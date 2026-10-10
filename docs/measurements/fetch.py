#!/usr/bin/env python3
"""Where a fetch goes, on the oracle and on Bund2, for each spelling of the
proxy variables and of the URL. Usage:

    python3 fetch.py [--port80] [--only F] target/oracle/release/bund target/debug/bund2

Run it from a scratch directory: it writes the programs it runs there, and
a directory `d` with a link in it. `--only` runs one section, A to G, with
the row numbers it has in the whole table. It
listens on 127.0.0.1 and ::1, on four ports the system picks and on 1080,
which is libcurl's default for a proxy and has to be free. With `--port80`
it also listens on port 80, which is where a client goes that has lost the
port it was given; macOS lets an ordinary user bind that port only on every
interface, so for the length of the run this machine answers on port 80.

Each listener answers an HTTP request with a Bund program that prints the
listener's name, and `use` evaluates it. A connection that is not HTTP is
sent the bare program after a second of silence. One listener, `p3`, refuses
a `CONNECT`. A cell is what was printed, then every connection any listener
received, in order, each with the credentials it carried:

    origin · origin:GET /lib.bund       fetched directly
    p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund    through a tunnel
    fail · p1:SOCKS5                    asked, in a protocol nobody here speaks
    origin · origin:other /lib.bund     asked in another protocol, and answered
    fail · -                            asked nobody
    p1 · p1:GET http://… [proxy-auth u:p]   with a proxy's credentials, decoded

`timeout` is a direct connection to an address nothing answers on. O, P1, P2
and P3 stand for the ports of the origin and the proxies.

A row about a response is run with `url`, not `use`, and its cell begins
with the text that came back: `body` for the program the listener holds,
`empty` for the empty string, the text itself for anything else, and `fail`
for a fetch that failed.
"""
import base64, os, socket, subprocess, sys, threading

LOG = []
SHOW_HOST = [False]

def extras(data):
    """The credentials a request carries, decoded, and its `Host` when a row
    asks for it. The request line alone cannot show either."""
    out = ""
    for h in data.split(b"\r\n")[1:]:
        name, _, value = h.partition(b":")
        name = name.strip().lower(); value = value.strip()
        if name in (b"authorization", b"proxy-authorization") and value[:6].lower() == b"basic ":
            try: cred = repr(base64.b64decode(value[6:]))[2:-1]
            except Exception: cred = "?" + value.decode("latin-1")
            out += " [%s %s]" % ("proxy-auth" if name[:1] == b"p" else "auth", cred)
        if name == b"host" and SHOW_HOST[0]:
            out += " [host %s]" % value.decode("latin-1")
    return out

def short(text):
    """A request line, or its first bytes and its length when it is long."""
    return text if len(text) < 200 else "%s… (%d bytes)" % (text[:24], len(text))

def first(data):
    """What a connection's first bytes are, when they are not an HTTP head."""
    if not data: return "silent"
    if data[:1] == b"\x05": return "SOCKS5"
    if data[:1] == b"\x04": return "SOCKS4"
    if data[:2] == b"\x16\x03": return "TLS"
    return "other " + repr(data[:24])[2:-1]

def chunks(body, size=None, end=b"\r\n", last=b"0"):
    """A body as one chunk and the closing one."""
    size = b"%x" % len(body) if size is None else size
    return size + end + body + end + last + end + end

OK = b"HTTP/1.1 200 OK\r\n"
CLOSE = b"Connection: close\r\n"
LEN = lambda b: b"Content-Length: %d\r\n" % len(b)
CHUNKED = b"Transfer-Encoding: chunked\r\n"
# A response by name, for the path `/r/<name>`: the whole of what is sent
# before the connection is closed.
SHAPES = {
 # A status that is not 200, with and without a length.
 "302-length": lambda b: b"HTTP/1.1 302 Found\r\nLocation: /lib.bund\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "302-no-length": lambda b: b"HTTP/1.1 302 Found\r\nLocation: /lib.bund\r\n" + CLOSE + b"\r\n" + b,
 "301-no-length": lambda b: b"HTTP/1.1 301 Moved\r\nLocation: /lib.bund\r\n" + CLOSE + b"\r\n" + b,
 "303-no-length": lambda b: b"HTTP/1.1 303 See Other\r\nLocation: /lib.bund\r\n" + CLOSE + b"\r\n" + b,
 "307-no-length": lambda b: b"HTTP/1.1 307 Temporary\r\nLocation: /lib.bund\r\n" + CLOSE + b"\r\n" + b,
 "308-no-length": lambda b: b"HTTP/1.1 308 Permanent\r\nLocation: /lib.bund\r\n" + CLOSE + b"\r\n" + b,
 "302-no-location": lambda b: b"HTTP/1.1 302 Found\r\n" + CLOSE + b"\r\n" + b,
 "300-no-length": lambda b: b"HTTP/1.1 300 Multiple\r\nLocation: /lib.bund\r\n" + CLOSE + b"\r\n" + b,
 "304-no-length": lambda b: b"HTTP/1.1 304 Not Modified\r\n" + CLOSE + b"\r\n" + b,
 "304-length": lambda b: b"HTTP/1.1 304 Not Modified\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "204-no-length": lambda b: b"HTTP/1.1 204 No Content\r\n" + CLOSE + b"\r\n" + b,
 "204-length": lambda b: b"HTTP/1.1 204 No Content\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "101": lambda b: b"HTTP/1.1 101 Switching\r\n" + CLOSE + b"\r\n" + b,
 "100-then-200": lambda b: b"HTTP/1.1 100 Continue\r\n\r\n" + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "103-then-200": lambda b: b"HTTP/1.1 103 Early Hints\r\nLink: </x>\r\n\r\n" + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "100-alone": lambda b: b"HTTP/1.1 100 Continue\r\n\r\n",
 "404": lambda b: b"HTTP/1.1 404 Not Found\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "500-no-length": lambda b: b"HTTP/1.1 500 Error\r\n" + CLOSE + b"\r\n" + b,
 "999": lambda b: b"HTTP/1.1 999 Odd\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "99": lambda b: b"HTTP/1.1 99 Odd\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "1000": lambda b: b"HTTP/1.1 1000 Odd\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 # The status line.
 "no-reason": lambda b: b"HTTP/1.1 200\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-1.0": lambda b: b"HTTP/1.0 200 OK\r\n" + LEN(b) + b"\r\n" + b,
 "http-1.0-no-length": lambda b: b"HTTP/1.0 200 OK\r\n\r\n" + b,
 "http-1.2": lambda b: b"HTTP/1.2 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-3": lambda b: b"HTTP/3 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "lower-case-http": lambda b: b"http/1.1 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "icy": lambda b: b"ICY 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "no-status-line": lambda b: b,
 "status-line-alone": lambda b: OK,
 "status-line-and-blank": lambda b: OK + b"\r\n",
 "nothing": lambda b: b"",
 "blank-line-first": lambda b: b"\r\n" + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "two-spaces-in-status": lambda b: b"HTTP/1.1  200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "tab-in-status": lambda b: b"HTTP/1.1\t200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 # The length.
 "no-length": lambda b: OK + CLOSE + b"\r\n" + b,
 "no-length-no-close": lambda b: OK + b"\r\n" + b,
 "length-0": lambda b: OK + b"Content-Length: 0\r\n" + CLOSE + b"\r\n" + b,
 "length-plus": lambda b: OK + b"Content-Length: +%d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-minus": lambda b: OK + b"Content-Length: -1\r\n" + CLOSE + b"\r\n" + b,
 "length-then-letters": lambda b: OK + b"Content-Length: %dabc\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-letters": lambda b: OK + b"Content-Length: abc\r\n" + CLOSE + b"\r\n" + b,
 "length-hex": lambda b: OK + b"Content-Length: 0x%x\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-past-64-bits": lambda b: OK + b"Content-Length: 99999999999999999999\r\n" + CLOSE + b"\r\n" + b,
 "length-empty": lambda b: OK + b"Content-Length:\r\n" + CLOSE + b"\r\n" + b,
 "length-blanks": lambda b: OK + b"Content-Length:   %d  \r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-leading-zeros": lambda b: OK + b"Content-Length: 000%d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-twice-same": lambda b: OK + LEN(b) + LEN(b) + CLOSE + b"\r\n" + b,
 "length-twice-short-first": lambda b: OK + b"Content-Length: 4\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "length-twice-long-first": lambda b: OK + LEN(b) + b"Content-Length: 4\r\n" + CLOSE + b"\r\n" + b,
 "length-list": lambda b: OK + b"Content-Length: %d, %d\r\n" % (len(b), len(b)) + CLOSE + b"\r\n" + b,
 "length-short": lambda b: OK + b"Content-Length: 4\r\n" + CLOSE + b"\r\n" + b,
 "length-long": lambda b: OK + b"Content-Length: %d\r\n" % (len(b) + 5) + CLOSE + b"\r\n" + b,
 # The transfer coding.
 "chunked": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "chunked-and-length": lambda b: OK + CHUNKED + b"Content-Length: 4\r\n" + CLOSE + b"\r\n" + chunks(b),
 "chunked-twice": lambda b: OK + CHUNKED + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "chunked-chunked": lambda b: OK + b"Transfer-Encoding: chunked, chunked\r\n" + CLOSE + b"\r\n" + chunks(b),
 "gzip-chunked": lambda b: OK + b"Transfer-Encoding: gzip, chunked\r\n" + CLOSE + b"\r\n" + chunks(b),
 "identity-coding": lambda b: OK + b"Transfer-Encoding: identity\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "unknown-coding": lambda b: OK + b"Transfer-Encoding: x\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "chunked-upper-case": lambda b: OK + b"TRANSFER-ENCODING: CHUNKED\r\n" + CLOSE + b"\r\n" + chunks(b),
 "chunked-http-1.0": lambda b: b"HTTP/1.0 200 OK\r\n" + CHUNKED + b"\r\n" + chunks(b),
 "chunk-size-plus": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"+%x" % len(b)),
 "chunk-size-blank-before": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b" %x" % len(b)),
 "chunk-size-blank-after": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"%x " % len(b)),
 "chunk-size-upper-case": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, (b"%X" % len(b))),
 "chunk-size-0x": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"0x%x" % len(b)),
 "chunk-size-leading-zeros": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"000%x" % len(b)),
 "chunk-size-17-digits": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"0" * 16 + b"%x" % len(b)),
 "chunk-extension": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"%x;a=b" % len(b)),
 "chunk-size-not-hex": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"zz"),
 "chunk-line-feeds": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, end=b"\n"),
 "chunk-no-last": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n",
 "chunk-short": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % (len(b) + 5) + b,
 "chunk-trailer": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T: y\r\n\r\n",
 "chunk-no-crlf-after-data": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"0\r\n\r\n",
 "chunk-two": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"4\r\n" + b[:4] + b"\r\n%x\r\n" % (len(b) - 4) + b[4:] + b"\r\n0\r\n\r\n",
 # The header lines.
 "line-feeds": lambda b: b"HTTP/1.1 200 OK\n" + b"Content-Length: %d\nConnection: close\n\n" % len(b) + b,
 "folded-header": lambda b: OK + b"X-A: one\r\n two\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "folded-length": lambda b: OK + b"Content-Length:\r\n %d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "blank-in-name": lambda b: OK + b"X A: y\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "blank-before-colon": lambda b: OK + b"Content-Length : %d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "empty-name": lambda b: OK + b": y\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "no-colon": lambda b: OK + b"X-A\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "carriage-return-in-value": lambda b: OK + b"X-A: one\rtwo\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "nul-in-value": lambda b: OK + b"X-A: one\x00two\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "high-byte-in-value": lambda b: OK + b"X-A: \xe9\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "high-byte-in-name": lambda b: OK + b"X-\xe9: y\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "header-100000-bytes": lambda b: OK + b"X-A: " + b"y" * 100000 + b"\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "header-400000-bytes": lambda b: OK + b"X-A: " + b"y" * 400000 + b"\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "gzip-content": lambda b: OK + b"Content-Encoding: gzip\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "keep-alive": lambda b: OK + LEN(b) + b"\r\n" + b,
 "bytes-after-body": lambda b: OK + LEN(b) + CLOSE + b"\r\n" + b + b"extra",
 "not-utf-8": lambda b: OK + b"Content-Length: %d\r\n" % (len(b) + 2) + CLOSE + b"\r\n" + b + b"\xff\n",
 # More of the status line.
 "blank-before-status": lambda b: b" " + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "http-2": lambda b: b"HTTP/2 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-2-no-length": lambda b: b"HTTP/2 200 OK\r\n" + CLOSE + b"\r\n" + b,
 "http-2-chunked": lambda b: b"HTTP/2 200 OK\r\n" + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "http-2-no-reason": lambda b: b"HTTP/2 200\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-2.0": lambda b: b"HTTP/2.0 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-2-two-digits": lambda b: b"HTTP/2 20 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-2-four-digits": lambda b: b"HTTP/2 2000 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-2-code-99": lambda b: b"HTTP/2 099 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-3-two-digits": lambda b: b"HTTP/3 20 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-3.0": lambda b: b"HTTP/3.0 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-4": lambda b: b"HTTP/4 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-1": lambda b: b"HTTP/1 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-0.9": lambda b: b"HTTP/0.9 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-1.1-no-blank": lambda b: b"HTTP/1.1200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "http-no-version": lambda b: b"HTTP/ 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "lower-case-http-304": lambda b: b"http/1.1 304 Not Modified\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "lower-case-http-1.2": lambda b: b"http/1.2 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "lower-case-http-chunked": lambda b: b"http/1.1 200 OK\r\n" + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "mixed-case-http": lambda b: b"Http/1.1 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "code-letters": lambda b: b"HTTP/1.1 abc OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "code-then-tab": lambda b: b"HTTP/1.1 200\tOK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "code-then-letter": lambda b: b"HTTP/1.1 200OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "code-099": lambda b: b"HTTP/1.1 099 Odd\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "code-000": lambda b: b"HTTP/1.1 000 Odd\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "nul-in-status": lambda b: b"HTTP/1.1 200 O\x00K\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "status-no-newline": lambda b: b"HTTP/1.1 200 OK",
 "status-line-feed-alone": lambda b: b"HTTP/1.1 200 OK\n",
 "three-bytes": lambda b: b"HTT",
 "five-bytes": lambda b: b"HTTP/",
 # Where the answer stops before its head does.
 "ends-after-length": lambda b: OK + LEN(b),
 "ends-after-length-0": lambda b: OK + b"Content-Length: 0\r\n",
 "ends-after-chunked": lambda b: OK + CHUNKED,
 "ends-in-a-header": lambda b: OK + b"X-A: y",
 "ends-in-a-header-after-length": lambda b: OK + LEN(b) + b"X-A",
 "ends-in-a-length": lambda b: OK + b"Content-Length: 5",
 # A provisional answer.
 "103-alone": lambda b: b"HTTP/1.1 103 Early Hints\r\n\r\n",
 "100-then-garbage": lambda b: b"HTTP/1.1 100 Continue\r\n\r\ngarbage\r\n\r\n",
 "100-twice-then-200": lambda b: b"HTTP/1.1 100 Continue\r\n\r\n" * 2 + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "100-length-then-200": lambda b: b"HTTP/1.1 100 Continue\r\nContent-Length: 5\r\n\r\n" + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "100-chunked-then-200": lambda b: b"HTTP/1.1 100 Continue\r\n" + CHUNKED + b"\r\n" + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "100-then-status-alone": lambda b: b"HTTP/1.1 100 Continue\r\n\r\n" + OK,
 "103-then-status-alone": lambda b: b"HTTP/1.1 103 Early Hints\r\n\r\n" + OK,
 "199-then-200": lambda b: b"HTTP/1.1 199 Odd\r\n\r\n" + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "100-then-lower-case-http": lambda b: b"HTTP/1.1 100 Continue\r\n\r\n" + b"http/1.1 200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "101-length": lambda b: b"HTTP/1.1 101 Switching\r\nContent-Length: 4\r\n" + CLOSE + b"\r\n" + b,
 "101-chunked": lambda b: b"HTTP/1.1 101 Switching\r\n" + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "204-chunked": lambda b: b"HTTP/1.1 204 No Content\r\n" + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "304-chunked": lambda b: b"HTTP/1.1 304 Not Modified\r\n" + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "204-bad-length": lambda b: b"HTTP/1.1 204 No Content\r\nContent-Length: abc\r\n" + CLOSE + b"\r\n" + b,
 "401-basic": lambda b: b"HTTP/1.1 401 No\r\nWWW-Authenticate: Basic realm=\"x\"\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "401-digest": lambda b: b"HTTP/1.1 401 No\r\nWWW-Authenticate: Digest realm=\"x\", nonce=\"abc\"\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "407-basic": lambda b: b"HTTP/1.1 407 No\r\nProxy-Authenticate: Basic realm=\"x\"\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 # More of the header lines.
 "first-header-folded": lambda b: OK + b" X-A: y\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "first-header-folded-no-colon": lambda b: OK + b" two\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "folded-with-colon": lambda b: OK + b"X-A: one\r\n Content-Length: 4\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "folded-length-after": lambda b: OK + LEN(b) + b" 5\r\n" + CLOSE + b"\r\n" + b,
 "header-begins-with-carriage-return": lambda b: OK + b"\rX-A: y\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "carriage-returns-alone": lambda b: b"HTTP/1.1 200 OK\r" + b"Content-Length: %d\rConnection: close\r\r" % len(b) + b,
 "length-lower-case": lambda b: OK + b"content-length: %d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-tab": lambda b: OK + b"Content-Length:\t%d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-no-blank": lambda b: OK + b"Content-Length:%d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-inner-blank": lambda b: OK + b"Content-Length: 1 7\r\n" + CLOSE + b"\r\n" + b,
 "length-plus-plus": lambda b: OK + b"Content-Length: ++%d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-plus-alone": lambda b: OK + b"Content-Length: +\r\n" + CLOSE + b"\r\n" + b,
 "length-minus-0": lambda b: OK + b"Content-Length: -0\r\n" + CLOSE + b"\r\n" + b,
 "length-largest": lambda b: OK + b"Content-Length: 9223372036854775807\r\n" + CLOSE + b"\r\n" + b,
 "length-largest-and-1": lambda b: OK + b"Content-Length: 9223372036854775808\r\n" + CLOSE + b"\r\n" + b,
 "length-vertical-tab": lambda b: OK + b"Content-Length: \x0b%d\r\n" % len(b) + CLOSE + b"\r\n" + b,
 "length-then-bad-length": lambda b: OK + LEN(b) + b"Content-Length: abc\r\n" + CLOSE + b"\r\n" + b,
 "length-prefix-name": lambda b: OK + b"Content-Length-X: 4\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 # More of the transfer coding.
 "length-then-chunked": lambda b: OK + b"Content-Length: 4\r\n" + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "chunked-then-gzip": lambda b: OK + b"Transfer-Encoding: chunked, gzip\r\n" + CLOSE + b"\r\n" + chunks(b),
 "x-then-chunked-two-lines": lambda b: OK + b"Transfer-Encoding: x\r\n" + CHUNKED + CLOSE + b"\r\n" + chunks(b),
 "chunked-then-x-two-lines": lambda b: OK + CHUNKED + b"Transfer-Encoding: x\r\n" + CLOSE + b"\r\n" + chunks(b),
 "identity-chunked": lambda b: OK + b"Transfer-Encoding: identity, chunked\r\n" + CLOSE + b"\r\n" + chunks(b),
 "chunked-parameter": lambda b: OK + b"Transfer-Encoding: chunked;q=1\r\n" + CLOSE + b"\r\n" + chunks(b),
 "comma-chunked": lambda b: OK + b"Transfer-Encoding: , chunked\r\n" + CLOSE + b"\r\n" + chunks(b),
 "chunked-comma": lambda b: OK + b"Transfer-Encoding: chunked,\r\n" + CLOSE + b"\r\n" + chunks(b),
 "chunked-no-blank": lambda b: OK + b"Transfer-Encoding:chunked\r\n" + CLOSE + b"\r\n" + chunks(b),
 "chunked-tab": lambda b: OK + b"Transfer-Encoding:\tchunked\t\r\n" + CLOSE + b"\r\n" + chunks(b),
 "chunkedx": lambda b: OK + b"Transfer-Encoding: chunkedx\r\n" + CLOSE + b"\r\n" + chunks(b),
 "coding-empty": lambda b: OK + b"Transfer-Encoding:\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "chunked-twice-and-twice": lambda b: OK + CHUNKED + CHUNKED + CLOSE + b"\r\n" + chunks(chunks(b)),
 "chunked-thrice-and-thrice": lambda b: OK + b"Transfer-Encoding: chunked, chunked\r\n" + CHUNKED + CLOSE + b"\r\n" + chunks(chunks(chunks(b))),
 "chunked-once-and-twice": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(chunks(b)),
 "chunk-size-16-digits": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"0" * 14 + b"%x" % len(b)),
 "chunk-size-empty": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b""),
 "chunk-size-then-letter": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"%xg" % len(b)),
 "chunk-size-tab-after": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"%x\t" % len(b)),
 "chunk-size-minus": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, b"-1"),
 "chunk-zero-alone": lambda b: OK + CHUNKED + CLOSE + b"\r\n0\r\n\r\n",
 "chunk-two-carriage-returns-after-data": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\r\n0\r\n\r\n",
 "chunk-last-00": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, last=b"00"),
 "chunk-last-extension": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b, last=b"0;a=b"),
 "chunk-last-line-feeds": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\n\n",
 "chunk-last-no-final-line": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\n",
 "chunk-last-final-carriage-return": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\n\r",
 "chunk-last-final-two-carriage-returns": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\n\r\r\n",
 "chunk-bytes-after-last": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + chunks(b) + b"extra",
 "chunk-trailer-no-colon": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T\r\n\r\n",
 "chunk-trailer-two": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T: y\r\nX-U: z\r\n\r\n",
 "chunk-trailer-line-feeds": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T: y\n\n",
 "chunk-trailer-folded": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T: y\r\n z\r\n\r\n",
 "chunk-trailer-begins-blank": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\n X-T: y\r\n\r\n",
 "chunk-trailer-nul": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T: \x00y\r\n\r\n",
 "chunk-trailer-carriage-return-inside": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T: y\rz\r\n\r\n",
 "chunk-trailer-unfinished": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T: y\r\n",
 "chunk-trailer-5000-bytes": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\nX-T: " + b"y" * 5000 + b"\r\n\r\n",
 # What the first measurement of these left open.
 "101-alone": lambda b: b"HTTP/1.1 101 Switching\r\n\r\n",
 "100-then-status-no-newline": lambda b: b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK",
 "204-ends-in-head": lambda b: b"HTTP/1.1 204 No Content\r\n",
 "line-feed-first": lambda b: b"\n" + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "http-2-tab": lambda b: b"HTTP/2\t200 OK\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "colon-after-carriage-return": lambda b: OK + b"X-A\rb: y\r\n" + LEN(b) + CLOSE + b"\r\n" + b,
 "chunk-trailer-empty-name": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\n: y\r\n\r\n",
 "chunk-trailer-blanks": lambda b: OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(b) + b + b"\r\n0\r\n  \r\n\r\n",
 "chunked-4-and-4": lambda b: OK + CHUNKED * 4 + CLOSE + b"\r\n" + chunks(chunks(chunks(chunks(b)))),
 "chunked-5-and-5": lambda b: OK + CHUNKED * 5 + CLOSE + b"\r\n" + chunks(chunks(chunks(chunks(chunks(b))))),
 "chunked-6-and-6": lambda b: OK + CHUNKED * 6 + CLOSE + b"\r\n" + chunks(chunks(chunks(chunks(chunks(chunks(b)))))),
 "chunked-twice-inner-unfinished": lambda b: OK + CHUNKED * 2 + CLOSE + b"\r\n" + chunks(b"%x\r\n" % len(b) + b + b"\r\n"),
 "chunked-twice-inner-bytes-after": lambda b: OK + CHUNKED * 2 + CLOSE + b"\r\n" + chunks(chunks(b) + b"extra"),
 "chunked-twice-outer-unfinished": lambda b: OK + CHUNKED * 2 + CLOSE + b"\r\n" + b"%x\r\n" % len(chunks(b)) + chunks(b) + b"\r\n",
 "chunked-twice-outer-bad-after-inner": lambda b: OK + CHUNKED * 2 + CLOSE + b"\r\n" + b"%x\r\n" % len(chunks(b)) + chunks(b) + b"\r\nzz\r\n",
 "100-12000-times": lambda b: b"HTTP/1.1 100 Continue\r\n\r\n" * 12000 + OK + LEN(b) + CLOSE + b"\r\n" + b,
 "100-12300-times": lambda b: b"HTTP/1.1 100 Continue\r\n\r\n" * 12300 + OK + LEN(b) + CLOSE + b"\r\n" + b,
}
def sized(name, body):
    """`/r/line-<n>`: one header line of n bytes, its line end among them.
    `/r/head-<n>`: a head of n bytes in all, from the status line to the
    empty line that ends it. `/r/trailer-<n>`: a trailer line of n bytes."""
    kind, _, n = name.partition("-")
    if not n.isdigit() or kind not in ("line", "head", "trailer"): return None
    n = int(n); tail = LEN(body) + CLOSE + b"\r\n"
    if kind == "line": return OK + b"X-A: " + b"y" * (n - 7) + b"\r\n" + tail + body
    if kind == "trailer": return OK + CHUNKED + CLOSE + b"\r\n" + b"%x\r\n" % len(body) + body + b"\r\n0\r\nX-T: " + b"y" * (n - 7) + b"\r\n\r\n"
    fill = n - len(OK) - len(tail)
    lines = b""
    while fill > 0:
        take = min(fill, 1000) if fill - min(fill, 1000) == 0 or fill - min(fill, 1000) >= 8 else fill - 8
        lines += b"X-A: " + b"y" * (take - 7) + b"\r\n"; fill -= take
    return OK + lines + tail + body

def answer(path, body):
    """The response for a path. `/h<n>` has n header lines in all, the two it
    needs among them, `/v2` a status line that names HTTP/2, and `/r/<name>`
    is one of `SHAPES`."""
    status, more = b"HTTP/1.1 200 OK", b""
    name = path.rsplit(b"/", 1)[-1]
    # A proxy is asked for the whole URL, and answers as the origin would.
    if path.startswith(b"http://"): path = b"/" + path.split(b"/", 3)[-1] if path.count(b"/") > 2 else b"/"
    if path.startswith(b"/r/"):
        if name.decode("latin-1") in SHAPES: return SHAPES[name.decode("latin-1")](body)
        made = sized(name.decode("latin-1"), body)
        if made: return made
    if name[:1] == b"h" and name[1:].isdigit():
        more = b"".join(b"X-%d: y\r\n" % i for i in range(int(name[1:]) - 2))
    if name == b"v2": status = b"HTTP/2 200 OK"
    return status + b"\r\n" + more + b"Content-Length: %d\r\nConnection: close\r\n\r\n" % len(body) + body

def serve(tag, port=0, fam=socket.AF_INET, host=None, refuse_connect=False):
    s = socket.socket(fam); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind((host or ("::1" if fam == socket.AF_INET6 else "127.0.0.1"), port)); s.listen(16)
    def head(c):
        """An HTTP head, or whatever arrived before the client stopped sending."""
        data = b""
        try:
            while b"\r\n\r\n" not in data:
                d = c.recv(4096)
                if not d: break
                data += d
        except OSError: pass
        return data
    def loop():
        while True:
            c, _ = s.accept()
            try:
                c.settimeout(1.0)
                data = head(c)
                line = data.split(b"\r\n")[0]
                body = ('"%s" println\n' % tag).encode()
                if not line.endswith(b" HTTP/1.1"):
                    # Not HTTP. Say what it was and answer with the bare
                    # program, which is all a gopher client wants.
                    LOG.append(tag + ":" + first(data)); c.sendall(body); c.close(); continue
                LOG.append(tag + ":" + short(line[:-9].decode("latin-1")) + extras(data))
                if line.startswith(b"CONNECT"):
                    if refuse_connect:
                        c.sendall(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"); c.close(); continue
                    c.sendall(b"HTTP/1.1 200 OK\r\n\r\n")
                    data = head(c); line = data.split(b"\r\n")[0]
                    LOG.append(tag + ":" + (line[:-9].decode("latin-1") + extras(data) if line.endswith(b" HTTP/1.1") else first(line)))
                c.sendall(answer(line.split(b" ")[1] if line.count(b" ") > 1 else b"", body))
            except OSError: pass
            c.close()
    threading.Thread(target=loop, daemon=True).start()
    return s.getsockname()[1]

args = sys.argv[1:]
only = None
if "--only" in args:
    i = args.index("--only"); only = args[i + 1]; del args[i:i + 2]
if "--port80" in args:
    args.remove("--port80"); serve("p80", 80, host="0.0.0.0")
bins = args
O = serve("origin"); serve("origin", O, socket.AF_INET6)
P1 = serve("p1"); P2 = serve("p2"); serve("p1080", 1080)
P3 = serve("p3", refuse_connect=True)
lib = os.path.abspath("lib file.bund"); open(lib, "w").write('"file" println\n')
# A directory reached through a link, with a different program above each end.
DIR = os.path.abspath("d"); os.makedirs(DIR + "/real/sub", exist_ok=True)
if not os.path.islink(DIR + "/link"): os.symlink(DIR + "/real/sub", DIR + "/link")
open(DIR + "/t.bund", "w").write('"file" println\n'); open(DIR + "/real/t.bund", "w").write('"linked" println\n')
open(DIR + "/data.txt", "w").write("x\n")
NAMES = ("origin", "p1", "p2", "p3", "p1080", "p80", "file", "linked")

def run(binp, url, env, host=False):
    e = {k: v for k, v in os.environ.items() if "proxy" not in k.lower()}
    sub = lambda t: t.replace("P1", str(P1)).replace("P2", str(P2)).replace("P3", str(P3)).replace("OPORT", str(O))
    e.update({k: sub(v) for k, v in env.items()})
    url = sub(url).replace("SPACE", lib).replace("LIB", lib.replace(" ", "%20")).replace("DIR", DIR)
    # A setting that begins `FILE ` is read by the `file` word, not by `use`;
    # `EVAL ` by `bund.eval-file`; and `TEXT ` by `url`, which is printed.
    text = url.startswith("TEXT ")
    # A long operand stands on a line of its own, so that a report of the
    # word's failure does not quote a megabyte of source (F186).
    sep = "\n" if len(url) > 100000 else " "
    if url.startswith("FILE "): prog = '"%s"%sfile bund.eval\n' % (url[5:], sep)
    elif url.startswith("EVAL "): prog = '"%s"%sbund.eval-file\n' % (url[5:], sep)
    elif text: prog = '"[" print%s"%s"%surl print "]" println\n' % (sep, url[5:], sep)
    else: prog = '"%s"%suse\n' % (url, sep)
    src = os.path.abspath("s.bund"); open(src, "w", encoding="utf-8").write(prog)
    LOG.clear(); SHOW_HOST[0] = host
    try:
        r = subprocess.run([binp, "script", "--file", src], env=e, capture_output=True, text=True, timeout=20, stdin=subprocess.DEVNULL)
        out = r.stdout.strip().splitlines(); out = out[-1] if out else "fail"
        if text:
            got = r.stdout
            if got.startswith("[") and got.endswith("]\n"):
                got = got[1:-2]
                out = "body" if got == '"origin" println\n' else "empty" if got == "" else "`%s`" % repr(got)[1:-1].replace("|", "&#124;")[:60]
            else: out = "fail"
        elif out not in NAMES: out = "fail"
    except subprocess.TimeoutExpired: out = "timeout"
    seen = ", ".join(LOG).replace(str(O), "O").replace(str(P1), "P1").replace(str(P2), "P2").replace(str(P3), "P3")
    return out + " · " + (seen or "-")

H = "127.0.0.1"; N = "x.bund2.invalid"
# A: which variable names the proxy, and what `no_proxy` takes away.
A = [
 ("none", H, {}),
 ("http_proxy", H, {"http_proxy": "http://127.0.0.1:P1"}),
 ("HTTP_PROXY", H, {"HTTP_PROXY": "http://127.0.0.1:P1"}),
 ("https_proxy", H, {"https_proxy": "http://127.0.0.1:P1"}),
 ("HTTPS_PROXY", H, {"HTTPS_PROXY": "http://127.0.0.1:P1"}),
 ("all_proxy", H, {"all_proxy": "http://127.0.0.1:P1"}),
 ("ALL_PROXY", H, {"ALL_PROXY": "http://127.0.0.1:P1"}),
 ("http_proxy=p1 all_proxy=p2", H, {"http_proxy": "http://127.0.0.1:P1", "all_proxy": "http://127.0.0.1:P2"}),
 ("http_proxy=p1 ALL_PROXY=p2", H, {"http_proxy": "http://127.0.0.1:P1", "ALL_PROXY": "http://127.0.0.1:P2"}),
 ("all_proxy=p1 ALL_PROXY=p2", H, {"all_proxy": "http://127.0.0.1:P1", "ALL_PROXY": "http://127.0.0.1:P2"}),
 ("http_proxy='' all_proxy=p2", H, {"http_proxy": "", "all_proxy": "http://127.0.0.1:P2"}),
 ("http_proxy no scheme", H, {"http_proxy": "127.0.0.1:P1"}),
 ("http_proxy garbage, all_proxy=p2", H, {"http_proxy": "http://", "all_proxy": "http://127.0.0.1:P2"}),
 ("unresolvable host via http_proxy", N, {"http_proxy": "http://127.0.0.1:P1"}),
 ("no_proxy=127.0.0.1", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1"}),
 ("NO_PROXY=127.0.0.1", H, {"http_proxy": "http://127.0.0.1:P1", "NO_PROXY": "127.0.0.1"}),
 ("no_proxy=other NO_PROXY=127.0.0.1", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "other.example", "NO_PROXY": "127.0.0.1"}),
 ("no_proxy=127.0.0.1 NO_PROXY=other", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1", "NO_PROXY": "other.example"}),
 ("no_proxy='' NO_PROXY=127.0.0.1", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "", "NO_PROXY": "127.0.0.1"}),
 ("no_proxy=*", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "*"}),
 ("no_proxy=a,*", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "a.example,*"}),
 ("no_proxy=a, 127.0.0.1 (space)", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "a.example, 127.0.0.1"}),
 ("no_proxy=a 127.0.0.1 (space only)", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "a.example 127.0.0.1"}),
 ("no_proxy=127.0.0.0/8", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.0/8"}),
 ("no_proxy=127.0.0.1:OPORT", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1:OPORT"}),
 ("no_proxy=127.0.0", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0"}),
 ("no_proxy=0.0.1", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "0.0.1"}),
 ("host x.bund2.invalid no_proxy=x.bund2.invalid", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "x.bund2.invalid"}),
 ("… no_proxy=X.BUND2.INVALID", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "X.BUND2.INVALID"}),
 ("… no_proxy=bund2.invalid", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "bund2.invalid"}),
 ("… no_proxy=.bund2.invalid", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": ".bund2.invalid"}),
 ("… no_proxy=*.bund2.invalid", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "*.bund2.invalid"}),
 ("… no_proxy=d2.invalid", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "d2.invalid"}),
 ("… no_proxy=invalid", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "invalid"}),
 ("… no_proxy=x.bund2.invalid.", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "x.bund2.invalid."}),
 ("… no_proxy=y.x.bund2.invalid", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "y.x.bund2.invalid"}),
 ("localhost no_proxy=LOCALHOST", "localhost", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "LOCALHOST"}),
 ("all_proxy='' ALL_PROXY=p2", H, {"all_proxy": "", "ALL_PROXY": "http://127.0.0.1:P2"}),
 ("http_proxy=socks5://p1", H, {"http_proxy": "socks5://127.0.0.1:P1"}),
 ("http_proxy=https://p1", H, {"http_proxy": "https://127.0.0.1:P1"}),
 ("http_proxy=ftp://p1", H, {"http_proxy": "ftp://127.0.0.1:P1"}),
 ("http_proxy=user:pw@p1", H, {"http_proxy": "http://u:p@127.0.0.1:P1"}),
 ("no_proxy=' * '", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": " * "}),
 ("no_proxy=127.0.0.0/0", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.0/0"}),
 ("no_proxy=127.0.0.1/0", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1/0"}),
 ("no_proxy=127.0.0.1/32", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1/32"}),
 ("no_proxy=127.0.0.1/33", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1/33"}),
 ("no_proxy=127.9.9.9/8", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.9.9.9/8"}),
 ("no_proxy=126.0.0.0/7", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "126.0.0.0/7"}),
 ("no_proxy=128.0.0.0/7", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "128.0.0.0/7"}),
 ("no_proxy=127.0.0.1/x", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1/x"}),
 ("no_proxy=.127.0.0.1", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": ".127.0.0.1"}),
 ("no_proxy=127.0.0.1.", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1."}),
 ("no_proxy=127.000.0.1", H, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.000.0.1"}),
 ("host [::1] no proxy vars", "[::1]", {}),
 ("host [::1] http_proxy", "[::1]", {"http_proxy": "http://127.0.0.1:P1"}),
 ("host [::1] no_proxy=::1", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::1"}),
 ("host [::1] no_proxy=[::1]", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "[::1]"}),
 ("host [::1] no_proxy=::/64", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::/64"}),
 ("host [::1] no_proxy=0:0:0:0:0:0:0:1", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "0:0:0:0:0:0:0:1"}),
 ("host [::1] no_proxy=::1/128", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::1/128"}),
 ("host [::1] no_proxy=::1/0", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::1/0"}),
 ("host [::1] no_proxy=::1/64", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::1/64"}),
 ("host [::1] no_proxy=::1/127", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::1/127"}),
 ("host [::1] no_proxy=::0/127", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::0/127"}),
 ("host [::1] no_proxy=::/8", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::/8"}),
 ("host [::1] no_proxy=0::1", "[::1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "0::1"}),
 ("host [0:0:0:0:0:0:0:1] no_proxy=::1", "[0:0:0:0:0:0:0:1]", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "::1"}),
 ("host x.bund2.invalid. no_proxy=bund2.invalid", N + ".", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "bund2.invalid"}),
 ("… no_proxy=..bund2.invalid", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "..bund2.invalid"}),
 ("… no_proxy=bund2.invalid..", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "bund2.invalid.."}),
 ("… no_proxy=. (dot alone)", N, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "."}),
 ("user@host no_proxy=127.0.0.1", "u:p@127.0.0.1", {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1"}),
]
# B: the proxy's own value, the URL's host, and the scheme's case.
U = "http://127.0.0.1:OPORT/lib.bund"
NP = {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.1"}
B = [(n, U, {"http_proxy": v}) for n, v in [
  ("proxy 127.0.0.1", "127.0.0.1"), ("proxy http://127.0.0.1", "http://127.0.0.1"), ("proxy localhost", "localhost"),
  ("proxy http://localhost", "http://localhost"), ("proxy http://127.0.0.1/", "http://127.0.0.1/"),
  ("proxy u:p@127.0.0.1", "u:p@127.0.0.1"), ("proxy 127.0.0.1:", "127.0.0.1:"), ("proxy http://127.0.0.1:/", "http://127.0.0.1:/"),
  ("proxy HTTP://127.0.0.1:P1", "HTTP://127.0.0.1:P1"), ("proxy http://127.0.0.1:P1/path", "http://127.0.0.1:P1/path"),
  ("proxy ' http://127.0.0.1:P1'", " http://127.0.0.1:P1"), ("proxy 'http://127.0.0.1:P1 '", "http://127.0.0.1:P1 "),
  ("proxy http://127.0.0.1:P1?q", "http://127.0.0.1:P1?q"), ("proxy http:127.0.0.1:P1", "http:127.0.0.1:P1"),
  ("proxy //127.0.0.1:P1", "//127.0.0.1:P1"), ("proxy http://127.1:P1", "http://127.1:P1"),
  ("proxy http://127.0.0.1:0", "http://127.0.0.1:0"), ("proxy http://127.0.0.1:99999", "http://127.0.0.1:99999"),
  ("proxy http://127.0.0.1:P1x", "http://127.0.0.1:P1x"), ("proxy socks5h://127.0.0.1", "socks5h://127.0.0.1"),
]]
for h in ["127.0.0.1", "127.1", "2130706433", "0x7f.0.0.1", "0177.0.0.1", "127.0.1", "0x7f000001", "017700000001", "127.0.0.01", "127.0.0.0x1", "0X7F.0.0.1", "127.0.0.1.", "127.0.0.09", "127.0.0.256", "127.0.65536", "127.0.0.1.1", "1.2.3.4.5", "127..1", "0", "localhost", "LocalHost"]:
    B.append(("host %s, no_proxy=127.0.0.1" % h, "http://%s:OPORT/lib.bund" % h, NP))
B.append(("host 127.1, no proxy at all", "http://127.1:OPORT/lib.bund", {}))
B.append(("host 127.0.0.1, no_proxy=127.1", U, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.1"}))
B.append(("host 127.0.0.1, no_proxy=0x7f.0.0.1", U, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "0x7f.0.0.1"}))
B.append(("host 127.0.0.1, no_proxy=2130706433", U, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "2130706433"}))
B.append(("host 127.0.0.1, no_proxy=0177.0.0.1", U, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "0177.0.0.1"}))
B.append(("host 127.0.0.1, no_proxy=127.0.0.010/32", U, {"http_proxy": "http://127.0.0.1:P1", "no_proxy": "127.0.0.010/32"}))
for sch in ["HTTP://", "Http://", "hTTp://"]:
    B.append(("url %s" % sch, sch + "127.0.0.1:OPORT/lib.bund", {}))
    B.append(("url %s via http_proxy" % sch, sch + "127.0.0.1:OPORT/lib.bund", {"http_proxy": "http://127.0.0.1:P1"}))
for u in ["file://LIB", "FILE://LIB", "File://LIB", "file://localhost" + "LIB", "file://LOCALHOST" + "LIB", "FILE://LocalHost" + "LIB", "file://127.0.0.1" + "LIB", "file://abs/lib.bund"]:
    B.append(("url " + u.replace("LIB", "/abs/lib%20file.bund"), u, {}))
# C: octal entries and other spellings of the URL.
PX = {"http_proxy": "http://127.0.0.1:P1"}
C = []
for host, np in [("127.0.0.10","127.0.0.010"),("127.0.0.8","127.0.0.010"),("127.0.0.10","127.0.0.012"),("127.0.0.1","127.0.0.01"),("127.0.0.1","127.0.0.1/8x"),("127.0.0.010","127.0.0.8"),("127.0.0.010","127.0.0.10"),("0x7f.1","127.0.0.0/8"),("127.0.0.1","127.0.0.1,"),("0","0.0.0.0")]:
    C.append(("host %s, no_proxy=%s" % (host, np), "http://%s:OPORT/lib.bund" % host, dict(PX, no_proxy=np)))
for u in ["http:/127.0.0.1:OPORT/lib.bund", "http:///127.0.0.1:OPORT/lib.bund", "http:////127.0.0.1:OPORT/lib.bund", "http:127.0.0.1:OPORT/lib.bund", "http:\\\\127.0.0.1:OPORT/lib.bund", " http://127.0.0.1:OPORT/lib.bund", "http://127.0.0.1:OPORT/lib.bund ", "http://127.0.0.1:OPORT/a b", "http://127.0.0.1:OPORT", "http://127.0.0.1:OPORT?x", "http://127.0.0.1:OPORT#f", "http://u:p@127.0.0.1:OPORT/x", "http://127.0.0.1:0OPORT/x", "127.0.0.1:OPORT/lib.bund", "//127.0.0.1:OPORT/lib.bund", "HTTPS://127.0.0.1:OPORT/x", "ftp://127.0.0.1:OPORT/x", "http://127.0.0.1:OPORT/%41?b=%20"]:
    C.append(("url " + u, u, {}))
for u in ["file:LIB", "file:///LIB", "file://localhost", "file://Localhost." + "LIB", "file://127.1" + "LIB", "file://[::1]" + "LIB", "file://localhost:80" + "LIB", "file://u@localhost" + "LIB", "file://LIB?x", "file://LIB#f", "file://LIB%00", "file://SPACE", "file:rel/lib.bund"]:
    C.append(("url " + u.replace("LIB", "/abs/lib%20file.bund").replace("SPACE", "/abs/lib file.bund"), u, {}))

# D: the port, the user part, and the path as it is sent.
D = []
T = "http://127.0.0.1:OPORT"
for path in ["/a/../lib.bund", "/a/./b", "/a/b/..", "/a/b/.", "/../x", "/a/../../x", "/a//b", "/./", "/..", "/a/%2e%2e/b", "/a/%2E/b", "/a/.%2e/b", "/a/..;x/b", "/a/...", "/a/..b/c", "/.a/b", "/a/../b?x=/../y", "/a/../b#f/../z", "/\u00e9.bund", "/a?q=\u00e9", "/%C3%A9", "/%zz", "/a\\\\b", "", "?x", "/x#frag", "/a/b/../../../c", "/a/./../b/", "/~x/$y,z;w=1@:!*'()", "/a\tb", "/\u00a0", "/x?\u00e9#\u00e9", "/a<b", "/a>b", "/a`b", "/a?<", "/a^b|c{d}e[f]"]:
    D.append(("path " + path.replace("\t", "<tab>").replace("\u00a0", "<nbsp>").replace("|", "&#124;"), T + path, {}))
for port in ["65536", "99999", "65616", "4294967376", "OPORTx", "0", "", "00OPORT", "+OPORT", "-1", " OPORT", "OPORT:OPORT"]:
    D.append(("url port :" + port.replace("OPORT", "O"), "http://127.0.0.1:%s/x" % port, {}))
D.append(("url port :65536, host localhost", "http://localhost:65536/x", {}))
D.append(("url port :65536, via http_proxy", "http://127.0.0.1:65536/x", {"http_proxy": "http://127.0.0.1:P1"}))
for auth in ["a@b@127.0.0.1:OPORT", "a:b:c@127.0.0.1:OPORT", "@127.0.0.1:OPORT", ":@127.0.0.1:OPORT", "a%40b@127.0.0.1:OPORT", "u s@127.0.0.1:OPORT", "[::ffff:127.0.0.1]:OPORT", "127.0.0.1.:OPORT", "LOCALHOST:OPORT", "local_host:OPORT", "-x:OPORT", ":OPORT", "127.0.0.%31:OPORT", "%6cocalhost:OPORT", "[::1%25lo0]:OPORT"]:
    D.append(("url http://" + auth.replace("OPORT", "O") + "/x", "http://" + auth + "/x", {}))
D.append(("url host b\u00fccher.invalid, via http_proxy", "http://b\u00fccher.invalid:OPORT/x", {"http_proxy": "http://127.0.0.1:P1"}))
for px in ["127.0.0.1:P1/", "127.0.0.1:P1/x", "127.0.0.1:P1?q", "localhost:P1/x", "127.0.0.1/", "127.0.0.1/x", "http://127.0.0.1:P1/x?q#f", "http://127.0.0.1:65536", "http://127.0.0.1:99999", "http://127.0.0.1:P1x", "127.0.0.1:99999", "http://127.0.0.1:0P1", "http://a@b@127.0.0.1:P1", "http://u:p@127.0.0.1:P1", "http://127.0.0.1:P1:1", "http://LOCALHOST:P1", "http://0x7f.1:P1", "http://[::ffff:127.0.0.1]:P1", "http://127.0.0.%31:P1", "https://127.0.0.1", "socks5://127.0.0.1"]:
    D.append(("proxy " + px, T + "/x", {"http_proxy": px}))

# E: schemes other than `file` and `http`.
E = []
for name, url, env in [
    ("gopher://", "gopher://127.0.0.1:OPORT/0/lib.bund", {}),
    ("gopher:// with http_proxy", "gopher://127.0.0.1:OPORT/0/lib.bund", {"http_proxy": "http://127.0.0.1:P1"}),
    ("dict://", "dict://127.0.0.1:OPORT/d:word", {}),
    ("ftp://", "ftp://127.0.0.1:OPORT/lib.bund", {}),
    ("ftp:// with ftp_proxy", "ftp://127.0.0.1:OPORT/lib.bund", {"ftp_proxy": "http://127.0.0.1:P1"}),
    ("ftp:// with all_proxy", "ftp://127.0.0.1:OPORT/lib.bund", {"all_proxy": "http://127.0.0.1:P1"}),
    ("ftp:// with http_proxy", "ftp://127.0.0.1:OPORT/lib.bund", {"http_proxy": "http://127.0.0.1:P1"}),
    ("telnet://", "telnet://127.0.0.1:OPORT", {}),
    ("https://", "https://127.0.0.1:OPORT/x", {}),
    ("https:// with https_proxy", "https://127.0.0.1:OPORT/x", {"https_proxy": "http://127.0.0.1:P1"}),
    ("ws://", "ws://127.0.0.1:OPORT/x", {}),
    ("smb://", "smb://127.0.0.1:OPORT/x/y", {}),
    ("tftp://", "tftp://127.0.0.1:OPORT/x", {}),
    ("no scheme, host:port/path", "127.0.0.1:OPORT/lib.bund", {}),
    ("no scheme, with http_proxy", "127.0.0.1:OPORT/lib.bund", {"http_proxy": "http://127.0.0.1:P1"}),
]:
    E.append(("scheme " + name, url, env))

# F: what a second parser would be handed unread — the host's bytes, the user
# part, a proxy's credentials, an entry's digits, an IPv6 host's text, a
# `file:` path's dot segments — and which responses are an answer.
F = []
for c in "!$&'()*+,;=|~_-":
    F.append(("host a%sb.invalid, via http_proxy" % c.replace("|", "&#124;"), "http://a%sb.invalid:OPORT/x" % c, PX))
for h in ["a!b.invalid", "a~b.invalid"]:
    F.append(("host %s, no proxy" % h, "http://%s:OPORT/x" % h, {}))
for h in ["127.0.0.1!", "127.0.0.1,1", "[zz]", "[::g]", "[1.2.3.4]", "[]", "[::1]x", "[:::1]", "[00000::1]", "[::ffff:1.2.3.04]"]:
    F.append(("host %s, via http_proxy" % h, "http://%s:OPORT/x" % h, PX))
for h in ["[0:0:0:0:0:0:0:1]", "[::0001]", "[0::0:1]", "[::FFFF:127.0.0.1]", "[::ffff:7f00:1]"]:
    F.append(("host %s, no proxy" % h, "http://%s:OPORT/x" % h, {}, True))
for h in ["[0:0:0:0:0:0:0:1]", "[0:0:0:0:0:0:0:2]", "[::2]", "[::ffff:7f00:1]", "[::FFFF:127.0.0.1]", "[0:0:0:0:0:0:0:A]", "[::A]", "[1:0:0:2:0:0:0:3]", "[1::2:0:0:0:3]", "[0:0:1::]", "[::1.2.3.4]", "[0:0:0:0:0:0:102:304]"]:
    F.append(("host %s, via http_proxy" % h, "http://%s:OPORT/x" % h, PX))
for h, np in [("[::ffff:7f00:1]", "::ffff:7f00:1"), ("[::ffff:7f00:1]", "::ffff:127.0.0.1"), ("[::FFFF:127.0.0.1]", "::ffff:127.0.0.1"), ("[::FFFF:127.0.0.1]", "0.1"), ("[::FFFF:127.0.0.1]", "127.0.0.1"), ("[0:0:0:0:0:0:0:1]", "0:0:0:0:0:0:0:1"), ("[::0001]", "::1"), ("[::0001]", "::0001"), ("[0:0:0:0:0:0:0:2]", "::2"), ("[0:0:0:0:0:0:0:2]", "::0.0.0.2"), ("[::2]", "::0.0.0.2")]:
    F.append(("host %s, no_proxy=%s" % (h, np), "http://%s:OPORT/x" % h, dict(PX, no_proxy=np)))
for np in ["0127.0.0.1", "00127.0.0.1", "127.0.0.0001", "127.0.0." + "0" * 30 + "1", "127.0.0.9/+8", "127.0.0.9/4294967304", "127.0.0.1/+33", "127.0.0.9/-8", "127.0.0.1/-0", "127.0.0.9/-0", "127.0.0.9/08", "127.0.0.1/4294967296", "127.0.0.9/4294967296", "127.0.0.9/4294967328", "127.0.0.1/99999999999999999999", "127.0.0.9/99999999999999999999", "127.0.0.1/-1", "127.0.0.1/+", "127.0.0.1/", "127.0.0.9/8/1", "127.0.0.1/+0x", "127.0.0.9/++8", "127.0.0.9/+-8", "127.0.0.9/-4294967288", "127.0.0.1/-99999999999999999999", "127.0.0.9/\n8", "0300.0.0.1"]:
    F.append(("no_proxy=" + np.replace("\n", "<newline>"), T + "/x", dict(PX, no_proxy=np)))
CREDS = ["u:p", "a%00b", "u:%00", "a%01b", "a%09b", "a%0ab", "a%0db", "a%1fb", "u:%1f", "a%20b", "a%7fb", "a%80b", "a%ffb", "a%40b:c%3Ad", "u%20s", "%zz", "a%", "a%4", "a<b", "a&#124;b", "\u00e9", "", ":", "u:", ":p", "a:b:c", "a;b", "u:p%2"]
for cred in CREDS:
    F.append(("url user part `%s@`" % cred, "http://%s@127.0.0.1:OPORT/x" % cred.replace("&#124;", "|"), {}))
for cred in ["u:p", "a%00b", "a%40b"]:
    F.append(("url user part `%s@`, via http_proxy" % cred, "http://%s@127.0.0.1:OPORT/x" % cred, PX))
for cred in CREDS:
    F.append(("proxy http://%s@127.0.0.1:P1" % cred, T + "/x", {"http_proxy": "http://%s@127.0.0.1:P1" % cred.replace("&#124;", "|")}))
F.append(("proxy a%00b@127.0.0.1:P1, no scheme", T + "/x", {"http_proxy": "a%00b@127.0.0.1:P1"}))
F.append(("all_proxy http://a%00b@127.0.0.1:P1", T + "/x", {"all_proxy": "http://a%00b@127.0.0.1:P1"}))
F.append(("http_proxy http://a%00b@127.0.0.1:P1, all_proxy=p2", T + "/x", {"http_proxy": "http://a%00b@127.0.0.1:P1", "all_proxy": "http://127.0.0.1:P2"}))
F.append(("proxy http://127.0.0.1:P1#f", T + "/x", {"http_proxy": "http://127.0.0.1:P1#f"}))
for u in ["file://DIR/link/../t.bund", "file://localhostDIR/link/../t.bund", "FILE DIR/link/../t.bund", "file://DIR/nosuch/../t.bund", "file://DIR/data.txt/../t.bund", "file://DIR/link/%2e%2e/t.bund", "file://DIR/real/sub/../t.bund", "file://DIR/link/./../t.bund", "file://DIR/./t.bund", "file://DIR/link/..", "FILE DIR/link/%2e%2e/t.bund"]:
    F.append(("url " + u.replace("DIR", "/abs/d").replace("FILE ", "the `file` word, "), u, {}))
for path in ["/h126", "/h127", "/h128", "/h129", "/h130", "/h200", "/v2", "/a?<>"]:
    F.append(("path " + path, T + path, {}))
F.append(("a proxy that refuses CONNECT and answers GET", T + "/x", {"http_proxy": "http://127.0.0.1:P3"}))

# G: the answer, which travels the other way; a `no_proxy` entry's length; a
# proxy's slashes; and `bund.eval-file`, the fetch's fourth caller.
G = []
for name in SHAPES:
    G.append(("response " + name, "TEXT " + T + "/r/" + name, {}))
for name in ["line-102398", "line-102399", "line-102400", "line-102401", "head-307198", "head-307199", "head-307200", "head-307201", "head-307202", "trailer-4095", "trailer-4096", "trailer-4097", "trailer-4098"]:
    G.append(("response " + name, "TEXT " + T + "/r/" + name, {}))
for path in ["/h4999", "/h5000", "/h5001", "/h5002", "/h5003"]:
    G.append(("path " + path, T + path, {}))
for name in ["401-basic", "401-digest", "404"]:
    G.append(("response %s, with a user part" % name, "TEXT http://u:p@127.0.0.1:OPORT/r/" + name, {}))
G.append(("response 407-basic, from a proxy with credentials", "TEXT " + T + "/r/407-basic", {"http_proxy": "http://u:p@127.0.0.1:P1"}))
for name in ["302-no-length", "101", "chunked-twice", "gzip-chunked", "length-twice-short-first", "status-line-alone"]:
    G.append(("response %s, by `use`" % name, T + "/r/" + name, {}))
for name in ["302-no-length", "chunked-twice", "chunked"]:
    G.append(("response %s, via http_proxy" % name, "TEXT " + T + "/r/" + name, PX))
for label, np in [("127.0.0.0/, 116 zeros, 8: 127 bytes", "127.0.0.0/" + "0" * 116 + "8"), ("127.0.0.0/, 117 zeros, 8: 128 bytes", "127.0.0.0/" + "0" * 117 + "8"),
                  ("127.0.0.1/ and 117 x: 127 bytes", "127.0.0.1/" + "x" * 117), ("127.0.0.1/ and 118 x: 128 bytes", "127.0.0.1/" + "x" * 118),
                  ("a name of 200 bytes, then 127.0.0.1", "a" * 200 + ",127.0.0.1"), ("127.0.0.1, then a name of 200 bytes", "127.0.0.1," + "a" * 200),
                  ("127.0.0.1/ and 118 x, then 127.0.0.1", "127.0.0.1/" + "x" * 118 + ",127.0.0.1")]:
    G.append(("no_proxy=" + label, T + "/x", dict(PX, no_proxy=np)))
G.append(("host x.bund2.invalid, no_proxy=a name of 200 bytes ending in it", "http://" + N + ":OPORT/x", dict(PX, no_proxy="a" * 185 + "." + N)))
G.append(("host of 200 bytes, no_proxy=its last 130", "http://" + "a" * 69 + "." + "b" * 121 + ".invalid:OPORT/x", dict(PX, no_proxy="b" * 121 + ".invalid")))
G.append(("host [::1], no_proxy=::1 after 127 zeros", "http://[::1]:OPORT/x", dict(PX, no_proxy="0" * 127 + "::1")))
for px in ["http:/127.0.0.1:P1", "http:///127.0.0.1:P1", "http:////127.0.0.1:P1", "HTTP:/127.0.0.1:P1", "127.0.0.1:P1/a://b", "localhost:P1/a://b", "x://127.0.0.1:P1", "x:/127.0.0.1:P1", "socks5:/127.0.0.1:P1", "http+x://127.0.0.1:P1", "1http://127.0.0.1:P1", "http:/u:p@127.0.0.1:P1", "http:/127.0.0.1", "http:\\\\127.0.0.1:P1", "http://127.0.0.1:P1\n", "\thttp://127.0.0.1:P1"]:
    G.append(("proxy " + px.replace("\n", "<newline>").replace("\t", "<tab>"), T + "/x", {"http_proxy": px}))
for u in ["EVAL DIR/t.bund", "EVAL d/t.bund", "EVAL LIB", "EVAL SPACE", "EVAL DIR/link/../t.bund", "EVAL DIR/t.bund?x", "EVAL DIR/t.bund#f", "EVAL DIR/link/%2e%2e/t.bund", "EVAL localhostDIR/t.bund", "EVAL /DIR/t.bund", "EVAL //DIR/t.bund", "EVAL DIR/nosuch.bund", "EVAL "]:
    G.append(("`bund.eval-file` of " + u[5:].replace("DIR", "/abs/d").replace("LIB", "/abs/lib%20file.bund").replace("SPACE", "/abs/lib file.bund"), u, {}))

# A request of n bytes in all, from `GET` to its empty line, and a URL of n.
def request_of(n, proxied):
    rest = "User-Agent: ZBUS\r\nAccept: */*\r\n" + ("Proxy-Connection: Keep-Alive\r\n" if proxied else "") + "\r\n"
    fixed = len("GET %s/ HTTP/1.1\r\nHost: 127.0.0.1:%d\r\n" % ("http://127.0.0.1:%d" % O if proxied else "", O) + rest)
    return "http://127.0.0.1:%d/" % O + "a" * (n - fixed)
# These are fetched with `url` and `file`, whose report of a failure does not
# quote the operand.
for n in [1048575, 1048576]:
    G.append(("a request of {:,} bytes".format(n), "TEXT " + request_of(n, False), {}))
    G.append(("a request of {:,} bytes, via http_proxy".format(n), "TEXT " + request_of(n, True), PX))
for n in [7999999, 8000000]:
    G.append(("an `http:` URL of {:,} bytes".format(n), "TEXT http://127.0.0.1:%d/" % O + "a" * (n - len("http://127.0.0.1:%d/" % O)), {}))
for n in [8000000, 8000001]:
    G.append(("the `file` word, a URL of {:,} bytes".format(n), "FILE DIR/t.bund?" + "a" * (n - len("file://" + DIR + "/t.bund?")), {}))

SECTIONS = {"A": [(n, "http://%s:OPORT/lib.bund" % h, env) for n, h, env in A], "B": B, "C": C, "D": D, "E": E, "F": F, "G": G}
cases = [c for k in "ABCDEFG" for c in SECTIONS[k]]
start = 1 + sum(len(SECTIONS[k]) for k in "ABCDEFG"[:"ABCDEFG".index(only)]) if only else 1
if only: cases = SECTIONS[only]
print("| # | setting | " + " | ".join(os.path.basename(b) for b in bins) + " |\n|---|---|" + "---|" * len(bins))
for i, (name, url, env, *host) in enumerate(cases, start):
    print("| %d | %s | %s |" % (i, name, " | ".join(run(b, url, env, bool(host)) for b in bins)), flush=True)
