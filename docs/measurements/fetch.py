#!/usr/bin/env python3
"""Where a fetch goes, on the oracle and on Bund2, for each spelling of the
proxy variables and of the URL. Usage:

    python3 fetch.py [--port80] [--only F] target/oracle/release/bund target/debug/bund2

Run it from a scratch directory: it writes the programs it runs there, and
a directory `d` with a link in it. `--only` runs one section, A to F, with
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

def first(data):
    """What a connection's first bytes are, when they are not an HTTP head."""
    if not data: return "silent"
    if data[:1] == b"\x05": return "SOCKS5"
    if data[:1] == b"\x04": return "SOCKS4"
    if data[:2] == b"\x16\x03": return "TLS"
    return "other " + repr(data[:24])[2:-1]

def answer(path, body):
    """The response for a path. `/h<n>` has n header lines in all, the two it
    needs among them, and `/v2` a status line that names HTTP/2."""
    status, more = b"HTTP/1.1 200 OK", b""
    name = path.rsplit(b"/", 1)[-1]
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
                LOG.append(tag + ":" + line[:-9].decode("latin-1") + extras(data))
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
    # A setting that begins `FILE ` is read by the `file` word, not by `use`.
    prog = '"%s" file bund.eval\n' % url[5:] if url.startswith("FILE ") else '"%s" use\n' % url
    src = os.path.abspath("s.bund"); open(src, "w", encoding="utf-8").write(prog)
    LOG.clear(); SHOW_HOST[0] = host
    try:
        r = subprocess.run([binp, "script", "--file", src], env=e, capture_output=True, text=True, timeout=20, stdin=subprocess.DEVNULL)
        out = r.stdout.strip().splitlines(); out = out[-1] if out else "fail"
        if out not in NAMES: out = "fail"
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

SECTIONS = {"A": [(n, "http://%s:OPORT/lib.bund" % h, env) for n, h, env in A], "B": B, "C": C, "D": D, "E": E, "F": F}
cases = [c for k in "ABCDEF" for c in SECTIONS[k]]
start = 1 + sum(len(SECTIONS[k]) for k in "ABCDEF"[:"ABCDEF".index(only)]) if only else 1
if only: cases = SECTIONS[only]
print("| # | setting | " + " | ".join(os.path.basename(b) for b in bins) + " |\n|---|---|" + "---|" * len(bins))
for i, (name, url, env, *host) in enumerate(cases, start):
    print("| %d | %s | %s |" % (i, name, " | ".join(run(b, url, env, bool(host)) for b in bins)), flush=True)
