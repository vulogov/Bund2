#!/usr/bin/env python3
"""Where a fetch goes, on the oracle and on Bund2, for each spelling of the
proxy variables and of the URL. Usage:

    python3 fetch.py [--port80] target/oracle/release/bund target/debug/bund2

Run it from a scratch directory: it writes the programs it runs there. It
listens on 127.0.0.1 and ::1, on three ports the system picks and on 1080,
which is libcurl's default for a proxy and has to be free. With `--port80`
it also listens on port 80, which is where a client goes that has lost the
port it was given; macOS lets an ordinary user bind that port only on every
interface, so for the length of the run this machine answers on port 80.

Each listener answers an HTTP request with a Bund program that prints the
listener's name, and `use` evaluates it. A connection that is not HTTP is
sent the bare program after a second of silence. A cell is what was printed, then
every connection any listener received, in order:

    origin · origin:GET /lib.bund       fetched directly
    p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund    through a tunnel
    fail · p1:SOCKS5                    asked, in a protocol nobody here speaks
    origin · origin:other /lib.bund     asked in another protocol, and answered
    fail · -                            asked nobody

`timeout` is a direct connection to an address nothing answers on. O, P1 and
P2 stand for the ports of the origin and the two proxies.
"""
import os, socket, subprocess, sys, threading

LOG = []

def first(data):
    """What a connection's first bytes are, when they are not an HTTP head."""
    if not data: return "silent"
    if data[:1] == b"\x05": return "SOCKS5"
    if data[:1] == b"\x04": return "SOCKS4"
    if data[:2] == b"\x16\x03": return "TLS"
    return "other " + repr(data[:24])[2:-1]

def serve(tag, port=0, fam=socket.AF_INET, host=None):
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
                LOG.append(tag + ":" + line[:-9].decode("latin-1"))
                if line.startswith(b"CONNECT"):
                    c.sendall(b"HTTP/1.1 200 OK\r\n\r\n")
                    line = head(c).split(b"\r\n")[0]
                    LOG.append(tag + ":" + (line[:-9].decode("latin-1") if line.endswith(b" HTTP/1.1") else first(line)))
                c.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: %d\r\nConnection: close\r\n\r\n" % len(body) + body)
            except OSError: pass
            c.close()
    threading.Thread(target=loop, daemon=True).start()
    return s.getsockname()[1]

args = sys.argv[1:]
if "--port80" in args:
    args.remove("--port80"); serve("p80", 80, host="0.0.0.0")
bins = args
O = serve("origin"); serve("origin", O, socket.AF_INET6)
P1 = serve("p1"); P2 = serve("p2"); serve("p1080", 1080)
lib = os.path.abspath("lib file.bund"); open(lib, "w").write('"file" println\n')
NAMES = ("origin", "p1", "p2", "p1080", "p80", "file")

def run(binp, url, env):
    e = {k: v for k, v in os.environ.items() if "proxy" not in k.lower()}
    sub = lambda t: t.replace("P1", str(P1)).replace("P2", str(P2)).replace("OPORT", str(O))
    e.update({k: sub(v) for k, v in env.items()})
    url = sub(url).replace("SPACE", lib).replace("LIB", lib.replace(" ", "%20"))
    src = os.path.abspath("s.bund"); open(src, "w", encoding="utf-8").write('"%s" use\n' % url)
    LOG.clear()
    try:
        r = subprocess.run([binp, "script", "--file", src], env=e, capture_output=True, text=True, timeout=20, stdin=subprocess.DEVNULL)
        out = r.stdout.strip().splitlines(); out = out[-1] if out else "fail"
        if out not in NAMES: out = "fail"
    except subprocess.TimeoutExpired: out = "timeout"
    seen = ", ".join(LOG).replace(str(O), "O").replace(str(P1), "P1").replace(str(P2), "P2")
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

cases = [(n, "http://%s:OPORT/lib.bund" % h, env) for n, h, env in A] + B + C + D + E
print("| # | setting | " + " | ".join(os.path.basename(b) for b in bins) + " |\n|---|---|" + "---|" * len(bins))
for i, (name, url, env) in enumerate(cases, 1):
    print("| %d | %s | %s |" % (i, name, " | ".join(run(b, url, env) for b in bins)), flush=True)
