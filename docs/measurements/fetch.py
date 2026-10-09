#!/usr/bin/env python3
"""Where a fetch goes, on the oracle and on Bund2, for each spelling of the
proxy variables and of the URL. Usage:

    python3 fetch.py target/oracle/release/bund target/debug/bund2

Run it from a scratch directory: it writes the programs it runs there. It
listens on 127.0.0.1 and ::1 only, on three ports the system picks and on
1080, which is libcurl's default for a proxy and has to be free.

Each listener answers any request with a Bund program that prints the
listener's name, and `use` evaluates it, so a cell is who answered: `origin`,
`p1`, `p2`, `p1080`, `file`, or `fail`. `timeout` is a direct connection to
an address nothing answers on. `/GET` and `/CONNECT` are what a proxy was
sent first.
"""
import os, socket, subprocess, sys, threading

def serve(tag, port=0, fam=socket.AF_INET):
    s = socket.socket(fam); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(("::1" if fam == socket.AF_INET6 else "127.0.0.1", port)); s.listen(16)
    seen = []
    def loop():
        while True:
            c, _ = s.accept()
            try:
                c.settimeout(2)
                while True:
                    data = b""
                    while b"\r\n\r\n" not in data:
                        d = c.recv(4096)
                        if not d: break
                        data += d
                    if not data: break
                    line = data.split(b"\r\n")[0].decode(errors="replace")
                    seen.append(line)
                    if line.startswith("CONNECT"):
                        c.sendall(b"HTTP/1.1 200 OK\r\n\r\n"); continue
                    body = ('"%s" println\n' % tag).encode()
                    c.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: %d\r\nConnection: close\r\n\r\n" % len(body) + body)
                    break
            except OSError: pass
            c.close()
    threading.Thread(target=loop, daemon=True).start()
    return s.getsockname()[1], seen

O, oseen = serve("origin"); serve("origin", O, socket.AF_INET6)
P1, p1seen = serve("p1"); P2, p2seen = serve("p2"); D, dseen = serve("p1080", 1080)
lib = os.path.abspath("lib file.bund"); open(lib, "w").write('"file" println\n')
bins = sys.argv[1:]

def run(binp, url, env):
    e = {k: v for k, v in os.environ.items() if "proxy" not in k.lower()}
    sub = lambda t: t.replace("P1", str(P1)).replace("P2", str(P2)).replace("OPORT", str(O))
    e.update({k: sub(v) for k, v in env.items()})
    url = sub(url).replace("SPACE", lib).replace("LIB", lib.replace(" ", "%20"))
    src = os.path.abspath("s.bund"); open(src, "w").write('"%s" use\n' % url)
    for l in (oseen, p1seen, p2seen, dseen): l.clear()
    try:
        r = subprocess.run([binp, "script", "--file", src], env=e, capture_output=True, text=True, timeout=20, stdin=subprocess.DEVNULL)
        out = r.stdout.strip().splitlines(); out = out[-1] if out else "fail"
        if out not in ("origin", "p1", "p2", "p1080", "file"): out = "fail"
    except subprocess.TimeoutExpired: out = "timeout"
    req = (p1seen or p2seen or dseen or [""])[0].split(" ")[0]
    return out + ("/" + req if req else "")

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

cases = [(n, "http://%s:OPORT/lib.bund" % h, env) for n, h, env in A] + B + C
print("| # | setting | " + " | ".join(os.path.basename(b) for b in bins) + " |\n|---|---|" + "---|" * len(bins))
for i, (name, url, env) in enumerate(cases, 1):
    print("| %d | %s | %s |" % (i, name, " | ".join(run(b, url, env) for b in bins)), flush=True)
