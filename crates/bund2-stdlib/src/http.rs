//! The HTTP exchange behind an `http:` fetch: one `GET`, written and read
//! here (D131).
//!
//! The reference hands a URL to libcurl
//! (`reference/Bund/src/stdlib/helpers/file_helper.rs:42-46`), so what goes
//! out and what counts as an answer are libcurl's. Bund2 used `ureq` for
//! both, and a second client is a second reader: of 94 shapes of response
//! tried against the oracle the two read 33 differently, in three directions
//! — a body the reference refuses was evaluated, a redirect with no length
//! was the empty string, and seventeen answers the reference reads failed
//! (F187). None of that is a setting of `ureq`'s. So the request is written
//! here, byte for byte as the oracle's was captured, and the response is
//! read by the rules below, **each one measured against the oracle**
//! (libcurl 8.7.1, macOS) on 2026-10-09: section G of
//! `docs/measurements/fetch.py`, 233 shapes of response.
//!
//! **The rules are a model, and the shapes are what it was checked on.** An
//! answer that combines them in a way nobody tried is read by the same
//! rules, and whether libcurl agrees there is not known.
//!
//! What is libcurl's and is not reproduced:
//!
//! - It tries a name's addresses side by side and this tries them in turn.
//! - With `chunked` named more than once it stops reading at the end of the
//!   piece of the connection in which a layer ended; this reads on to the
//!   end of the outermost layer, and so may refuse what follows.
//! - Neither sets a limit on how long an answer may take.

use std::io::{Read, Write};

/// The longest line of a head, its line feed counted. One byte more and
/// libcurl gives up on the answer.
const LINE_MAX: usize = 102_399;
/// The most bytes every head of one answer may hold together, provisional
/// ones included, from each status line to the empty line that ends it.
const HEAD_MAX: usize = 307_200;
/// The longest trailer line after a chunked body, without its line end.
const TRAILER_MAX: usize = 4093;
/// How many times `chunked` may be named. Each naming is one more layer of
/// framing to take off.
const LAYERS_MAX: usize = 4;
/// The most bytes a request may be, from `GET` to the empty line that ends
/// it. libcurl connects, finds a larger one does not fit, and sends nothing.
const REQUEST_MAX: usize = 1_048_575;

/// One `GET`.
pub(crate) struct Request<'a> {
    /// Where the connection goes: the origin, or the proxy. A host in
    /// brackets is an IPv6 address.
    pub connect: (&'a str, u16),
    /// The URL's host, with its port when that is not 80. It is what `Host`
    /// carries.
    pub host: &'a str,
    /// The path and the query.
    pub target: &'a str,
    /// The bytes for `Authorization: Basic`.
    pub login: Option<&'a [u8]>,
    /// `Some` when `connect` is a proxy, holding the bytes for
    /// `Proxy-Authorization: Basic` if it has credentials.
    pub proxy: Option<Option<&'a [u8]>>,
}

impl Request<'_> {
    /// The request as libcurl writes it for the reference, captured from the
    /// oracle: a proxy is asked for the whole URL and told
    /// `Proxy-Connection: Keep-Alive`, the header lines are in this order,
    /// and there are no others. `None` if a part holds a byte that would end
    /// its line; the callers have refused such a URL already.
    fn head(&self) -> Option<Vec<u8>> {
        use base64ct::Encoding as _;
        let breaks = |text: &str| text.bytes().any(|b| b <= b' ' || b == 0x7f);
        if breaks(self.host) || breaks(self.target) {
            return None;
        }
        let mut head = String::from("GET ");
        if self.proxy.is_some() {
            head.push_str("http://");
            head.push_str(self.host);
        }
        head.push_str(self.target);
        head.push_str(" HTTP/1.1\r\nHost: ");
        head.push_str(self.host);
        head.push_str("\r\n");
        if let Some(Some(login)) = self.proxy {
            head.push_str("Proxy-Authorization: Basic ");
            head.push_str(&base64ct::Base64::encode_string(login));
            head.push_str("\r\n");
        }
        if let Some(login) = self.login {
            head.push_str("Authorization: Basic ");
            head.push_str(&base64ct::Base64::encode_string(login));
            head.push_str("\r\n");
        }
        head.push_str("User-Agent: ZBUS\r\nAccept: */*\r\n");
        if self.proxy.is_some() {
            head.push_str("Proxy-Connection: Keep-Alive\r\n");
        }
        head.push_str("\r\n");
        Some(head.into_bytes())
    }
}

/// Send `request` and return the body of the answer. `None` is a fetch that
/// failed, for any reason.
///
/// **A request past [`REQUEST_MAX`] is not sent, and the connection is made
/// all the same**: that is the order libcurl does it in, and a listener saw
/// the oracle connect and say nothing for a URL of a megabyte. Measured for
/// a request with no proxy, through one, and with credentials; the limit is
/// on the whole request in each.
pub(crate) fn get(request: &Request) -> Option<Vec<u8>> {
    let head = request.head()?;
    let (host, port) = request.connect;
    let host = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host);
    let mut stream = std::net::TcpStream::connect((host, port)).ok()?;
    if head.len() > REQUEST_MAX {
        return None;
    }
    stream.write_all(&head).ok()?;
    read_response(&mut stream)
}

/// The bytes of an answer, one at a time.
struct Wire<'a> {
    from: &'a mut dyn Read,
    held: Vec<u8>,
    at: usize,
    ended: bool,
}

/// A line of a head.
enum Line {
    /// Through its line feed.
    Whole(Vec<u8>),
    /// The answer ended before the line did.
    Cut,
}

impl<'a> Wire<'a> {
    fn new(from: &'a mut dyn Read) -> Self {
        Self { from, held: Vec::new(), at: 0, ended: false }
    }

    /// The next byte, or `Some(None)` where the answer ends. `None` is a
    /// connection that failed.
    fn byte(&mut self) -> Option<Option<u8>> {
        if self.at >= self.held.len() {
            if self.ended {
                return Some(None);
            }
            self.held.resize(16 * 1024, 0);
            // An interrupted read is tried again, and not for ever.
            let mut got = None;
            for _ in 0..64 {
                match self.from.read(&mut self.held) {
                    Ok(n) => {
                        got = Some(n);
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_) => return None,
                }
            }
            self.held.truncate(got?);
            self.at = 0;
            if self.held.is_empty() {
                self.ended = true;
                return Some(None);
            }
        }
        let byte = self.held.get(self.at).copied();
        self.at += 1;
        Some(byte)
    }

    /// The next line. `None` is a failed connection, or a line past
    /// [`LINE_MAX`].
    fn line(&mut self) -> Option<Line> {
        let mut line = Vec::new();
        // One turn more than the longest line, to see what follows it.
        for _ in 0..=LINE_MAX {
            let Some(byte) = self.byte()? else {
                return Some(Line::Cut);
            };
            if line.len() == LINE_MAX {
                break;
            }
            line.push(byte);
            if byte == b'\n' {
                return Some(Line::Whole(line));
            }
        }
        None
    }

    /// Everything up to where the answer ends.
    fn rest(&mut self) -> Option<Vec<u8>> {
        let mut body = Vec::new();
        while let Some(byte) = self.byte()? {
            body.push(byte);
        }
        Some(body)
    }
}

/// C's `isspace`, which is what libcurl asks of the byte after a number.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// The status a status line gives the answer. `None` is a line libcurl gives
/// up on.
///
/// - The line begins `HTTP/` in any case, with nothing before it, and holds
///   no zero byte.
/// - **In upper case it is read strictly.** `HTTP/1.0` or `HTTP/1.1`, one
///   blank, three digits, and white space after them — or the answer is
///   refused. `HTTP/2` and `HTTP/3` take the same, without the minor
///   version; a status below 100 is refused, and so is a well-formed
///   `HTTP/3` line. Any other version is refused.
/// - **Anything else that begins `HTTP/` is status 200**: `http/1.1 304`,
///   `Http/1.1`, `HTTP/2.0 200` and `HTTP/2 20 OK` all are, whatever number
///   they carry. libcurl takes the prefix for an alias of a plain success.
fn status(line: &[u8]) -> Option<u16> {
    if !line.get(..5)?.eq_ignore_ascii_case(b"HTTP/") || line.contains(&0) {
        return None;
    }
    if line.get(..5) != Some(b"HTTP/") {
        return Some(200);
    }
    let blank = |at: usize| matches!(line.get(at), Some(b' ' | b'\t'));
    let space = |at: usize| line.get(at).copied().is_some_and(is_space);
    let code = |at: usize| {
        let digits = line.get(at..at + 3)?;
        digits
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| digits.iter().fold(0u16, |n, d| n * 10 + u16::from(d - b'0')))
    };
    match line.get(5).copied()? {
        b'1' => {
            let minor = line.get(6) == Some(&b'.') && matches!(line.get(7), Some(b'0' | b'1'));
            code(9).filter(|code| minor && blank(8) && space(12) && *code >= 100)
        }
        version @ (b'2' | b'3') => match code(7).filter(|_| blank(6) && space(10)) {
            Some(code) if code < 100 || version == b'3' => None,
            Some(code) => Some(code),
            None => Some(200),
        },
        _ => None,
    }
}

/// What a `Content-Length` line says.
enum Length {
    Is(u64),
    /// A number past 63 bits. The line is passed over, and whatever length
    /// was known before it stands.
    Past,
    /// Not a number libcurl reads. The answer is refused.
    Bad,
}

/// A `Content-Length` value as libcurl reads it: blanks, an optional `+`,
/// digits, and nothing asked of what follows them. So `12abc` is 12, `1 7`
/// is 1 and `0x11` is 0; `-0`, `++1`, a value with no digit and an empty one
/// are refused.
fn content_length(value: &[u8]) -> Length {
    let from = value.iter().position(|b| !matches!(b, b' ' | b'\t')).unwrap_or(value.len());
    let value = value.get(from..).unwrap_or_default();
    let digits = match value.first() {
        Some(b'+') => value.get(1..).unwrap_or_default(),
        _ => value,
    };
    let count = digits.iter().take_while(|b| b.is_ascii_digit()).count();
    if count == 0 {
        return Length::Bad;
    }
    let mut n = 0u64;
    for digit in digits.iter().take(count) {
        let next = n.checked_mul(10).and_then(|n| n.checked_add(u64::from(digit - b'0')));
        match next {
            Some(next) if next <= i64::MAX as u64 => n = next,
            _ => return Length::Past,
        }
    }
    Length::Is(n)
}

/// How many times a `Transfer-Encoding` value names `chunked` before it
/// names anything else. **libcurl stops reading the list at the first coding
/// it was not asked to undo**, so `gzip, chunked` names it no times and the
/// body is taken as it arrives, framing and all, while `chunked, gzip` names
/// it once. A name is what stands between commas with the blanks before it
/// and the white space after it dropped, in any case; `chunked;q=1` is
/// another name.
fn chunked_layers(value: &[u8]) -> usize {
    let mut layers = 0;
    for name in value.split(|b| *b == b',') {
        let from = name.iter().position(|b| !matches!(b, b' ' | b'\t')).unwrap_or(name.len());
        let to = name.iter().rposition(|b| !is_space(*b)).map_or(0, |last| last + 1);
        let Some(name) = name.get(from..to) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        if !name.eq_ignore_ascii_case(b"chunked") {
            break;
        }
        layers += 1;
    }
    layers
}

/// Whether libcurl keeps a header line, which it does twice over: once
/// looking at the line, and once filing it.
///
/// - A zero byte refuses it.
/// - It holds a colon, unless it begins with a blank and is not the first
///   line under its status line: that is a folded line, and it continues
///   the one before.
/// - **The colon stands before the line's first carriage return.** The
///   filing reads a line up to there, so `X-A\rb: y` has no name.
///
/// `filed` is whether any line of this answer was filed before, which is
/// what a folded line continues.
fn header_is_kept(line: &[u8], first: bool, filed: &mut bool) -> bool {
    let folded = matches!(line.first(), Some(b' ' | b'\t'));
    if line.contains(&0) || ((first || !folded) && !line.contains(&b':')) {
        return false;
    }
    if folded && *filed {
        return true;
    }
    let end = line.iter().position(|b| matches!(b, b'\r' | b'\n')).unwrap_or(line.len());
    *filed = true;
    line.get(..end).is_some_and(|text| text.contains(&b':'))
}

/// The value of the header `name` — written with its colon — if `line` is
/// that header. The name is matched in any case from the line's first byte,
/// so a folded line is never one, and neither is `Content-Length : 5`.
fn header<'a>(line: &'a [u8], name: &[u8]) -> Option<&'a [u8]> {
    let (head, value) = line.split_at_checked(name.len())?;
    head.eq_ignore_ascii_case(name).then_some(value)
}

/// Where libcurl is in one layer of chunked framing. The names are its own.
#[derive(Clone, Copy, PartialEq)]
enum ChunkState {
    Hex,
    Lf,
    Data,
    PostLf,
    Trailer,
    TrailerCr,
    TrailerPostCr,
    Stop,
    Done,
}

/// One layer of chunked framing, taken off a byte at a time as libcurl
/// takes it off.
///
/// - **A size** is one to sixteen hex digits and nothing before them: no
///   sign, no blank, no `0x`. Whatever follows the digits on the line is
///   passed over, an extension or not.
/// - **A line ends at its line feed.** A carriage return before it is
///   optional everywhere, and after a chunk's data any number of them may
///   stand.
/// - **A trailer** is a line of at most [`TRAILER_MAX`] bytes with a colon,
///   or one that begins with a blank, and holds no zero byte and no
///   carriage return of its own.
/// - **The body is over at the line feed of the empty line after the last
///   chunk**, and what the connection carries after that is not read.
struct Dechunker {
    state: ChunkState,
    digits: u8,
    size: u64,
    trailer: Vec<u8>,
}

impl Dechunker {
    fn new() -> Self {
        Self { state: ChunkState::Hex, digits: 0, size: 0, trailer: Vec::new() }
    }

    fn done(&self) -> bool {
        self.state == ChunkState::Done
    }

    /// Take one byte of framing. `Some(Some(b))` is a byte of the body under
    /// it, and `None` is framing libcurl refuses.
    fn feed(&mut self, byte: u8) -> Option<Option<u8>> {
        // Some states pass a byte on unread to the next. Each does so once,
        // and none passes one back, so eight turns is every state there is.
        for _ in 0..8 {
            match self.state {
                ChunkState::Hex => {
                    if let Some(digit) = (byte as char).to_digit(16) {
                        if self.digits >= 16 {
                            return None;
                        }
                        self.size = self.size << 4 | u64::from(digit);
                        self.digits += 1;
                        return Some(None);
                    }
                    if self.digits == 0 || self.size > i64::MAX as u64 {
                        return None;
                    }
                    self.state = ChunkState::Lf;
                }
                ChunkState::Lf => {
                    if byte == b'\n' {
                        self.state = if self.size == 0 { ChunkState::Trailer } else { ChunkState::Data };
                    }
                    return Some(None);
                }
                ChunkState::Data => {
                    self.size = self.size.saturating_sub(1);
                    if self.size == 0 {
                        self.state = ChunkState::PostLf;
                    }
                    return Some(Some(byte));
                }
                ChunkState::PostLf => {
                    if byte == b'\n' {
                        self.state = ChunkState::Hex;
                        self.digits = 0;
                    } else if byte != b'\r' {
                        return None;
                    }
                    return Some(None);
                }
                ChunkState::Trailer => {
                    if byte != b'\r' && byte != b'\n' {
                        self.trailer.push(byte);
                        return (self.trailer.len() <= TRAILER_MAX + 2).then_some(None);
                    }
                    if self.trailer.is_empty() {
                        self.state = ChunkState::TrailerPostCr;
                        continue;
                    }
                    let folded = matches!(self.trailer.first(), Some(b' ' | b'\t'));
                    if self.trailer.len() > TRAILER_MAX
                        || self.trailer.contains(&0)
                        || !(folded || self.trailer.contains(&b':'))
                    {
                        return None;
                    }
                    self.trailer.clear();
                    self.state = ChunkState::TrailerCr;
                    if byte == b'\r' {
                        return Some(None);
                    }
                }
                ChunkState::TrailerCr => {
                    if byte != b'\n' {
                        return None;
                    }
                    self.state = ChunkState::TrailerPostCr;
                    return Some(None);
                }
                ChunkState::TrailerPostCr => {
                    if byte != b'\r' && byte != b'\n' {
                        self.state = ChunkState::Trailer;
                        continue;
                    }
                    self.state = ChunkState::Stop;
                    if byte == b'\r' {
                        return Some(None);
                    }
                }
                ChunkState::Stop => {
                    if byte != b'\n' {
                        return None;
                    }
                    self.state = ChunkState::Done;
                    return Some(None);
                }
                ChunkState::Done => return Some(None),
            }
        }
        None
    }
}

/// A body under `layers` layers of chunked framing.
///
/// One layer is the ordinary case. More than one is `chunked` named more
/// than once, and libcurl then takes the framing off that many times: a
/// body framed once under two namings fails, at the first byte of it that
/// is not a hex digit. **The answer is complete when any layer has seen its
/// end**, so with two, the inner one may stop short if the outer one ends,
/// and the outer one may stop short if the inner one ended. Reading stops
/// when the outermost has.
fn dechunked(wire: &mut Wire, layers: usize) -> Option<Vec<u8>> {
    let mut layers: Vec<Dechunker> = (0..layers).map(|_| Dechunker::new()).collect();
    let mut body = Vec::new();
    while !layers.first().is_some_and(Dechunker::done) {
        let Some(byte) = wire.byte()? else {
            return layers.iter().any(Dechunker::done).then_some(body);
        };
        let mut carried = Some(byte);
        for layer in &mut layers {
            let Some(byte) = carried else { break };
            carried = layer.feed(byte)?;
        }
        body.extend(carried);
    }
    Some(body)
}

/// Read an answer and return its body, as libcurl 8.7.1 returns it to the
/// reference. `None` is an answer it gives up on.
///
/// **The head.** Lines end at a line feed, and the head ends at a line that
/// begins with a carriage return or a line feed — all of that line, so a
/// header written `\rX: y` ends the head and is lost. [`status`] reads the
/// first line and [`header_is_kept`] each of the others. No line is longer
/// than [`LINE_MAX`], and all the heads together hold no more than
/// [`HEAD_MAX`] bytes.
///
/// **A status from 100 to 199 is not the answer**, and another head
/// follows; that is how the loop is bounded, by [`HEAD_MAX`]. `101` is the
/// exception: its head is the last, and what follows it is the body, to the
/// end of the connection.
///
/// **Whether there is a body.** A provisional status, `204` and `304` have
/// none to describe, so their `Content-Length` and `Transfer-Encoding` are
/// not read at all — not even to refuse a bad one. `204` and `304` are the
/// empty string. Every other status has a body, a redirect's included, and
/// the reference follows no redirect: a `302` is its own body.
///
/// **How long the body is**, first rule that applies:
///
/// 1. `chunked` was named: [`dechunked`], whatever the version of HTTP and
///    whatever `Content-Length` says.
/// 2. A `Content-Length` was read: that many bytes, and fewer is a failure.
///    The last line read wins, each one must be a number ([`content_length`])
///    and one past 63 bits is passed over.
/// 3. Otherwise everything to the end of the connection.
///
/// **An answer that ends inside its head** is the empty string, and not a
/// failure, once one whole line of that head has arrived — unless a
/// `Content-Length` above zero was among them. With no whole line it is a
/// failure, and that holds for the head after a provisional one too.
pub(crate) fn read_response(from: &mut dyn Read) -> Option<Vec<u8>> {
    let mut wire = Wire::new(from);
    let mut head_bytes = 0usize;
    let mut filed = false;
    // Every turn reads a status line of six bytes or more out of HEAD_MAX.
    for _ in 0..HEAD_MAX {
        let Line::Whole(line) = wire.line()? else {
            return None;
        };
        head_bytes += line.len();
        let code = status(&line).filter(|_| head_bytes <= HEAD_MAX)?;
        let described = !matches!(code, 100..=199 | 204 | 304);
        let mut length = None;
        let mut layers = 0;
        let mut first = true;
        loop {
            let Line::Whole(line) = wire.line()? else {
                return length.is_none_or(|n| n == 0).then(Vec::new);
            };
            head_bytes += line.len();
            if head_bytes > HEAD_MAX {
                return None;
            }
            if matches!(line.first(), Some(b'\r' | b'\n')) {
                break;
            }
            if !header_is_kept(&line, first, &mut filed) {
                return None;
            }
            first = false;
            if !described {
                continue;
            }
            if let Some(value) = header(&line, b"Content-Length:") {
                match content_length(value) {
                    Length::Is(n) => length = Some(n),
                    Length::Past => {}
                    Length::Bad => return None,
                }
            } else if let Some(value) = header(&line, b"Transfer-Encoding:") {
                layers += chunked_layers(value);
                if layers > LAYERS_MAX {
                    return None;
                }
            }
        }
        return match code {
            101 => wire.rest().filter(|body| !body.is_empty()),
            100..=199 => continue,
            204 | 304 => Some(Vec::new()),
            _ if layers > 0 => dechunked(&mut wire, layers),
            _ => match length {
                Some(n) => {
                    let mut body = Vec::new();
                    while (body.len() as u64) < n {
                        body.push(wire.byte()??);
                    }
                    Some(body)
                }
                None => wire.rest(),
            },
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: &[u8] = b"\"origin\" println\n";
    const OK: &[u8] = b"HTTP/1.1 200 OK\r\n";
    const CHUNKED: &[u8] = b"Transfer-Encoding: chunked\r\n";

    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    /// `Content-Length` for the body, and the end of the head.
    fn len() -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n", B.len()).into_bytes()
    }

    /// The body as one chunk and the closing one.
    fn chunks(body: &[u8]) -> Vec<u8> {
        cat(&[format!("{:x}\r\n", body.len()).as_bytes(), body, b"\r\n0\r\n\r\n"])
    }

    /// One chunk, with the body's size written as `size`, and then `after`.
    fn chunk(size: &str, after: &[u8]) -> Vec<u8> {
        cat(&[OK, CHUNKED, b"\r\n", size.as_bytes(), b"\r\n", B, after])
    }

    fn read(answer: &[u8]) -> Option<Vec<u8>> {
        let mut from = answer;
        read_response(&mut from)
    }

    /// A status line, then the body with its length.
    fn after(status: &str) -> Option<Vec<u8>> {
        read(&cat(&[status.as_bytes(), b"\r\n", &len(), B]))
    }

    /// One header line before the length and the body.
    fn with(header: &[u8]) -> Option<Vec<u8>> {
        read(&cat(&[OK, header, &len(), B]))
    }

    /// A `Content-Length` value, before the body.
    fn sized(value: &str) -> Option<Vec<u8>> {
        read(&cat(&[OK, format!("Content-Length:{value}\r\n\r\n").as_bytes(), B]))
    }

    /// A `Transfer-Encoding` value, before the body as one chunk.
    fn coded(value: &str) -> Option<Vec<u8>> {
        read(&cat(&[OK, format!("Transfer-Encoding:{value}\r\n\r\n").as_bytes(), &chunks(B)]))
    }

    fn body() -> Option<Vec<u8>> {
        Some(B.to_vec())
    }

    fn empty() -> Option<Vec<u8>> {
        Some(Vec::new())
    }

    /// Every row here is one the oracle answered this way on 2026-10-09:
    /// section G of `docs/measurements/fetch.py`.
    #[test]
    fn a_status_line_is_read_as_libcurl_reads_it() {
        for status in [
            "HTTP/1.1 200 OK", "HTTP/1.0 200 OK", "HTTP/1.1 200", "HTTP/1.1\t200 OK", "HTTP/1.1 200\tOK",
            "HTTP/1.1 404 Not Found", "HTTP/1.1 999 Odd", "HTTP/2 200 OK", "HTTP/2\t200 OK", "HTTP/2 200",
            // Not a status line libcurl parses, and so status 200.
            "http/1.1 200 OK", "Http/1.1 200 OK", "http/1.2 200 OK", "http/1.1 304 Not Modified",
            "HTTP/2.0 200 OK", "HTTP/2 20 OK", "HTTP/2 2000 OK", "HTTP/3 20 OK", "HTTP/3.0 200 OK",
        ] {
            assert_eq!(after(status), body(), "{status}");
        }
        for status in [
            "HTTP/1.1 99 Odd", "HTTP/1.1 1000 Odd", "HTTP/1.1 099 Odd", "HTTP/1.1 000 Odd", "HTTP/1.2 200 OK",
            "HTTP/3 200 OK", "HTTP/4 200 OK", "HTTP/1 200 OK", "HTTP/0.9 200 OK", "HTTP/ 200 OK",
            "HTTP/1.1200 OK", "HTTP/1.1  200 OK", "HTTP/1.1 abc OK", "HTTP/1.1 200OK", "HTTP/2 099 OK",
            "ICY 200 OK", " HTTP/1.1 200 OK", "HTTP/1.1 200 O\0K", "",
        ] {
            assert_eq!(after(status), None, "{status:?}");
        }
        assert_eq!(read(b""), None);
        assert_eq!(read(B), None, "no status line");
        assert_eq!(read(b"HTT"), None);
        assert_eq!(read(b"HTTP/"), None);
        assert_eq!(read(b"HTTP/1.1 200 OK"), None, "no line feed");
        assert_eq!(read(&cat(&[b"\n", OK, &len(), B])), None, "a line before it");
    }

    #[test]
    fn a_status_decides_whether_a_body_follows() {
        let moved = |status: &str| read(&cat(&[status.as_bytes(), b"\r\nLocation: /x\r\n\r\n", B]));
        // A redirect is its own body, with a length or without: the
        // fifteenth review's B1.
        for status in ["HTTP/1.1 300 M", "HTTP/1.1 301 M", "HTTP/1.1 302 Found", "HTTP/1.1 303 S", "HTTP/1.1 307 T", "HTTP/1.1 308 P"] {
            assert_eq!(moved(status), body(), "{status}");
            assert_eq!(after(status), body(), "{status}, with a length");
        }
        assert_eq!(read(&cat(&[b"HTTP/1.1 500 Error\r\n\r\n", B])), body());
        // 204 and 304 have none, whatever they say of one.
        for status in ["HTTP/1.1 204 No Content", "HTTP/1.1 304 Not Modified"] {
            assert_eq!(moved(status), empty(), "{status}");
            assert_eq!(after(status), empty(), "{status}, with a length");
            let said = |header: &str| read(&cat(&[status.as_bytes(), b"\r\n", header.as_bytes(), b"\r\n\r\n", &chunks(B)]));
            assert_eq!(said("Transfer-Encoding: chunked"), empty(), "{status}");
            assert_eq!(said("Content-Length: abc"), empty(), "{status}: a bad length is not read");
        }
        // A provisional answer is passed over, and what it says of a body is
        // not read.
        let then = |first: &str| read(&cat(&[first.as_bytes(), OK, &len(), B]));
        assert_eq!(then("HTTP/1.1 100 Continue\r\n\r\n"), body());
        assert_eq!(then("HTTP/1.1 103 Early Hints\r\nLink: </x>\r\n\r\n"), body());
        assert_eq!(then("HTTP/1.1 199 Odd\r\n\r\n"), body());
        assert_eq!(then("HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 100 Continue\r\n\r\n"), body());
        assert_eq!(then("HTTP/1.1 100 Continue\r\nContent-Length: 5\r\n\r\n"), body());
        assert_eq!(then("HTTP/1.1 100 Continue\r\nTransfer-Encoding: chunked\r\n\r\n"), body());
        assert_eq!(read(&cat(&[b"HTTP/1.1 100 Continue\r\n\r\nhttp/1.1 200 OK\r\n", &len(), B])), body());
        assert_eq!(read(b"HTTP/1.1 100 Continue\r\n\r\n"), None);
        assert_eq!(read(b"HTTP/1.1 103 Early Hints\r\n\r\n"), None);
        assert_eq!(read(b"HTTP/1.1 100 Continue\r\n\r\ngarbage\r\n\r\n"), None);
        assert_eq!(read(b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK"), None);
        assert_eq!(read(b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\n"), empty());
        // 101 ends the heads, and its body runs to the end of the connection.
        assert_eq!(read(&cat(&[b"HTTP/1.1 101 Switching\r\n\r\n", B])), body());
        assert_eq!(read(&cat(&[b"HTTP/1.1 101 Switching\r\nContent-Length: 4\r\n\r\n", B])), body());
        assert_eq!(read(&cat(&[b"HTTP/1.1 101 S\r\n", CHUNKED, b"\r\n", &chunks(B)])), Some(chunks(B)));
        assert_eq!(read(b"HTTP/1.1 101 Switching\r\n\r\n"), None);
        // The heads together are bounded, so the provisional ones are.
        let many = |n: usize| read(&cat(&[&b"HTTP/1.1 100 Continue\r\n\r\n".repeat(n), OK, &len(), B]));
        assert_eq!(many(12_000), body());
        assert_eq!(many(12_300), None);
    }

    #[test]
    fn a_length_is_read_as_libcurl_reads_it() {
        for value in [" 17", "17", "\t17", "   17  ", " 00017", " +17", " 17abc", " 17, 17"] {
            assert_eq!(sized(value), body(), "{value:?}");
        }
        assert_eq!(sized(" 0"), empty());
        assert_eq!(sized(" 0x11"), empty(), "the digits before the x");
        assert_eq!(sized(" 4"), Some(b"\"ori".to_vec()));
        assert_eq!(sized(" 1 7"), Some(b"\"".to_vec()));
        // Past 63 bits is no length at all, and the body runs to the end.
        assert_eq!(sized(" 9223372036854775808"), body());
        assert_eq!(sized(" 99999999999999999999"), body());
        for value in [" -1", " -0", " abc", "", " ", " ++17", " +", " \x0b17", " 22", " 9223372036854775807"] {
            assert_eq!(sized(value), None, "{value:?}");
        }
        let two = |a: &str, b: &str| read(&cat(&[OK, format!("Content-Length: {a}\r\nContent-Length: {b}\r\n\r\n").as_bytes(), B]));
        assert_eq!(two("4", "17"), body(), "the last one read wins");
        assert_eq!(two("17", "4"), Some(b"\"ori".to_vec()));
        assert_eq!(two("17", "abc"), None, "and every one is read");
        assert_eq!(two("4", "99999999999999999999"), Some(b"\"ori".to_vec()), "one past 63 bits is passed over");
        // The name, in any case and from the line's first byte.
        assert_eq!(read(&cat(&[OK, b"content-length: 17\r\n\r\n", B, b"extra"])), body());
        assert_eq!(read(&cat(&[OK, b"Content-Length : 4\r\n\r\n", B])), body(), "not the header");
        assert_eq!(read(&cat(&[OK, b"Content-Length-X: 4\r\n\r\n", B])), body(), "not the header");
        assert_eq!(read(&cat(&[OK, b"\r\n", B])), body(), "no length: to the end");
        assert_eq!(read(&cat(&[b"HTTP/1.0 200 OK\r\n\r\n", B])), body());
        assert_eq!(read(&cat(&[b"HTTP/1.1 200 OK\nContent-Length: 17\n\n", B])), body(), "line feeds alone");
    }

    #[test]
    fn a_head_that_stops_short_is_the_empty_string_or_a_failure() {
        assert_eq!(read(OK), empty());
        assert_eq!(read(b"HTTP/1.1 200 OK\n"), empty());
        assert_eq!(read(&cat(&[OK, b"\r\n"])), empty());
        assert_eq!(read(&cat(&[OK, b"X-A: y"])), empty());
        assert_eq!(read(&cat(&[OK, b"Content-Length: 5"])), empty(), "a line that never ended was never read");
        assert_eq!(read(&cat(&[OK, b"Content-Length: 0\r\n"])), empty());
        assert_eq!(read(&cat(&[OK, CHUNKED])), empty());
        assert_eq!(read(b"HTTP/1.1 204 No Content\r\n"), empty());
        assert_eq!(read(&cat(&[OK, b"Content-Length: 17\r\n"])), None);
        assert_eq!(read(&cat(&[OK, b"Content-Length: 17\r\nX-A"])), None);
        // All one line: a carriage return ends nothing.
        assert_eq!(read(&cat(&[b"HTTP/1.1 200 OK\rContent-Length: 17\r\r", B])), empty());
    }

    #[test]
    fn a_header_line_is_kept_or_refuses_the_answer() {
        for header in [
            &b"X-A: one\r\n two\r\n"[..], b"X A: y\r\n", b": y\r\n", b"X-A: one\rtwo\r\n", b"X-A: \xe9\r\n",
            b"X-\xe9: y\r\n", b" X-A: y\r\n", b"X-A: one\r\n Content-Length: 4\r\n",
            b"Content-Encoding: gzip\r\n", b"Transfer-Encoding: x\r\n", b"Transfer-Encoding: identity\r\n",
            b"Transfer-Encoding:\r\n", b"WWW-Authenticate: Basic realm=\"x\"\r\n",
        ] {
            assert_eq!(with(header), body(), "{:?}", String::from_utf8_lossy(header));
        }
        for header in [&b"X-A\r\n"[..], b"X-A: one\0two\r\n", b" two\r\n", b"X-A\rb: y\r\n", b"Content-Length:\r\n 17\r\n"] {
            assert_eq!(with(header), None, "{:?}", String::from_utf8_lossy(header));
        }
        // A line that begins with a carriage return ends the head, and all
        // of that line goes with it.
        assert_eq!(with(b"\rX-A: y\r\n"), Some(cat(&[&len(), B])));
        // A folded line after the length does not change it.
        assert_eq!(read(&cat(&[OK, b"Content-Length: 17\r\n 5\r\n\r\n", B])), body());
        // A line of LINE_MAX bytes is read and one byte more is not; and
        // HEAD_MAX bytes of head are read and one more is not.
        let line = |n: usize| with(&cat(&[b"X-A: ", &b"y".repeat(n - 7), b"\r\n"]));
        assert_eq!(line(LINE_MAX), body());
        assert_eq!(line(LINE_MAX + 1), None);
        let head = |n: usize| {
            let fill = n - OK.len() - len().len();
            let lines = cat(&[&b"X-A: yyy\r\n".repeat(fill / 10 - 1), b"X-A: ", &b"y".repeat(fill % 10 + 3), b"\r\n"]);
            assert_eq!(OK.len() + lines.len() + len().len(), n);
            with(&lines)
        };
        assert_eq!(head(HEAD_MAX), body());
        assert_eq!(head(HEAD_MAX + 1), None);
    }

    #[test]
    fn chunked_is_named_or_the_body_is_taken_as_it_arrives() {
        for value in [" chunked", "chunked", "\tchunked\t", " CHUNKED", " chunked, gzip", " , chunked", " chunked,"] {
            assert_eq!(coded(value), body(), "{value:?}");
        }
        // The list is read up to the first coding that is not `chunked`.
        for value in [" gzip, chunked", " identity, chunked", " chunked;q=1", " chunkedx", " x"] {
            assert_eq!(coded(value), Some(chunks(B)), "{value:?}");
        }
        let lines = |head: &[u8], body: &[u8]| read(&cat(&[OK, head, b"\r\n", body]));
        assert_eq!(lines(b"Transfer-Encoding: x\r\nTransfer-Encoding: chunked\r\n", &chunks(B)), body());
        assert_eq!(lines(b"Transfer-Encoding: chunked\r\nTransfer-Encoding: x\r\n", &chunks(B)), body());
        // It outranks a length, before it or after, and HTTP/1.0.
        assert_eq!(lines(b"Content-Length: 4\r\nTransfer-Encoding: chunked\r\n", &chunks(B)), body());
        assert_eq!(lines(b"Transfer-Encoding: chunked\r\nContent-Length: 4\r\n", &chunks(B)), body());
        assert_eq!(read(&cat(&[b"HTTP/1.0 200 OK\r\n", CHUNKED, b"\r\n", &chunks(B)])), body());
        assert_eq!(read(&cat(&[b"http/1.1 200 OK\r\n", CHUNKED, b"\r\n", &chunks(B)])), body());
        // Named twice it is taken off twice: the fifteenth review's B1.
        let twice = cat(&[CHUNKED, CHUNKED]);
        assert_eq!(lines(&twice, &chunks(B)), None);
        assert_eq!(coded(" chunked, chunked"), None);
        assert_eq!(lines(&twice, &chunks(&chunks(B))), body());
        assert_eq!(lines(CHUNKED, &chunks(&chunks(B))), Some(chunks(B)));
        let nested = |n: usize| (0..n).fold(B.to_vec(), |inner, _| chunks(&inner));
        assert_eq!(lines(&CHUNKED.repeat(3), &nested(3)), body());
        assert_eq!(lines(&CHUNKED.repeat(LAYERS_MAX), &nested(LAYERS_MAX)), body());
        assert_eq!(lines(&CHUNKED.repeat(LAYERS_MAX + 1), &nested(LAYERS_MAX + 1)), None);
        // Either layer's end is the end.
        let inner_open = chunks(&cat(&[b"11\r\n", B, b"\r\n"]));
        assert_eq!(lines(&twice, &inner_open), body());
        let outer_open = cat(&[format!("{:x}\r\n", chunks(B).len()).as_bytes(), &chunks(B), b"\r\n"]);
        assert_eq!(lines(&twice, &outer_open), body());
        assert_eq!(lines(&twice, &cat(&[&outer_open, b"zz\r\n"])), None);
        assert_eq!(lines(&twice, &chunks(&cat(&[&chunks(B), b"extra"]))), body());
    }

    #[test]
    fn chunked_framing_is_taken_off_as_libcurl_takes_it_off() {
        let end = b"\r\n0\r\n\r\n";
        for size in ["11", "11 ", "11\t", "0011", "00000000000011", "11;a=b", "11g"] {
            assert_eq!(read(&chunk(size, end)), body(), "{size:?}");
        }
        assert_eq!(read(&cat(&[OK, CHUNKED, b"\r\n11\n", B, b"\n0\n\n"])), body(), "line feeds alone");
        assert_eq!(read(&cat(&[OK, CHUNKED, b"\r\n4\r\n\"ori\r\nd\r\ngin\" println\n\r\n0\r\n\r\n"])), body());
        assert_eq!(read(&cat(&[OK, CHUNKED, b"\r\n0\r\n\r\n"])), empty());
        for size in ["+11", " 11", "0x11", "000000000000000011", "zz", "", "-1"] {
            assert_eq!(read(&chunk(size, end)), None, "{size:?}");
        }
        for after in [
            &b"\r\n0\r\n\r\n"[..], b"\r\r\n0\r\n\r\n", b"\n0\n\n", b"\r\n00\r\n\r\n", b"\r\n0;a=b\r\n\r\n",
            b"\r\n0\r\n\r\nextra", b"\r\n0\r\nX-T: y\r\n\r\n", b"\r\n0\r\nX-T: y\r\nX-U: z\r\n\r\n",
            b"\r\n0\r\nX-T: y\n\n", b"\r\n0\r\nX-T: y\r\n z\r\n\r\n", b"\r\n0\r\n X-T: y\r\n\r\n",
            b"\r\n0\r\n: y\r\n\r\n", b"\r\n0\r\n  \r\n\r\n",
        ] {
            assert_eq!(read(&chunk("11", after)), body(), "{:?}", String::from_utf8_lossy(after));
        }
        for after in [
            &b""[..], b"\r\n", b"0\r\n\r\n", b"\r\n0\r\n", b"\r\n0\r\n\r", b"\r\n0\r\n\r\r\n",
            b"\r\n0\r\nX-T\r\n\r\n", b"\r\n0\r\nX-T: \0y\r\n\r\n", b"\r\n0\r\nX-T: y\rz\r\n\r\n",
            b"\r\n0\r\nX-T: y\r\n",
        ] {
            assert_eq!(read(&chunk("11", after)), None, "{:?}", String::from_utf8_lossy(after));
        }
        assert_eq!(read(&chunk("16", b"")), None, "a chunk cut short");
        // A trailer of TRAILER_MAX bytes is read and one byte more is not.
        let trailer = |n: usize| read(&chunk("11", &cat(&[b"\r\n0\r\nX-T: ", &b"y".repeat(n - 5), b"\r\n\r\n"])));
        assert_eq!(trailer(TRAILER_MAX), body());
        assert_eq!(trailer(TRAILER_MAX + 1), None);
    }

    /// The request, as the oracle's was captured on 2026-10-09.
    #[test]
    fn a_request_is_written_as_libcurl_writes_it() {
        let text = |request: Request| request.head().map(|head| String::from_utf8_lossy(&head).into_owned());
        let plain = Request { connect: ("h", 80), host: "127.0.0.1:8080", target: "/a?b", login: None, proxy: None };
        assert_eq!(
            text(plain).as_deref(),
            Some("GET /a?b HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nUser-Agent: ZBUS\r\nAccept: */*\r\n\r\n")
        );
        let both = Request {
            connect: ("p", 1080),
            host: "example.invalid",
            target: "/a",
            login: Some(b"u:p"),
            proxy: Some(Some(b"x:y")),
        };
        assert_eq!(
            text(both).as_deref(),
            Some(
                "GET http://example.invalid/a HTTP/1.1\r\nHost: example.invalid\r\n\
                 Proxy-Authorization: Basic eDp5\r\nAuthorization: Basic dTpw\r\n\
                 User-Agent: ZBUS\r\nAccept: */*\r\nProxy-Connection: Keep-Alive\r\n\r\n"
            )
        );
        let through = Request { connect: ("p", 1080), host: "[::1]:9", target: "/", login: None, proxy: Some(None) };
        assert_eq!(
            text(through).as_deref(),
            Some(
                "GET http://[::1]:9/ HTTP/1.1\r\nHost: [::1]:9\r\nUser-Agent: ZBUS\r\nAccept: */*\r\n\
                 Proxy-Connection: Keep-Alive\r\n\r\n"
            )
        );
        // A request of REQUEST_MAX bytes is sent and one byte more is not,
        // though the listener is connected to either way.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
        let port = listener.local_addr().expect("addr").port();
        let seen = std::thread::spawn(move || {
            let mut sizes = Vec::new();
            for conn in listener.incoming().take(2) {
                let mut conn = conn.expect("connection");
                let mut got = Vec::new();
                let mut piece = [0u8; 65536];
                while !got.ends_with(b"\r\n\r\n") {
                    match conn.read(&mut piece) {
                        Ok(n) if n > 0 => got.extend_from_slice(&piece[..n]),
                        _ => break,
                    }
                }
                sizes.push(got.len());
                let _ = conn.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi");
            }
            sizes
        });
        let host = format!("127.0.0.1:{port}");
        let fixed = format!("GET  HTTP/1.1\r\nHost: {host}\r\nUser-Agent: ZBUS\r\nAccept: */*\r\n\r\n").len();
        let ask = |size: usize| {
            let target = format!("/{}", "a".repeat(size - fixed - 1));
            get(&Request { connect: ("127.0.0.1", port), host: &host, target: &target, login: None, proxy: None })
        };
        assert_eq!(ask(REQUEST_MAX), Some(b"hi".to_vec()));
        assert_eq!(ask(REQUEST_MAX + 1), None);
        assert_eq!(seen.join().expect("listener"), vec![REQUEST_MAX, 0]);

        for (host, target) in [("h", "/a b"), ("h", "/a\r\nX: y"), ("h\n", "/"), ("h", "/\x7f")] {
            let request = Request { connect: ("h", 80), host, target, login: None, proxy: None };
            assert_eq!(request.head(), None, "{host:?} {target:?}");
        }
    }
}
