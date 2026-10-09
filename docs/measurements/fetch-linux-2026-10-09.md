# Where a fetch goes on Linux: the oracle and Bund2, 443 settings

Measured 2026-10-09 on `ubuntu-latest`: Ubuntu 24.04, x86-64, kernel
6.17.0-1022-azure. The oracle is `reference/Bund` at `21b40b0`, built out
of tree by the workflow, and it links the system's libcurl dynamically
(`libcurl.so.4`, 8.5.0). Bund2 is `17e623a`. The run is workflow
`measure-fetch`, 38002480092; `fetch.py` printed the table below exactly,
with `--port80`. *(The first version of this file was run 37994971955, of
Bund2 at `bbd50c1`. Seven cells differ, all Bund2's, and are named below.)* It is the companion of `fetch-2026-10-09.md`, which is
macOS arm64 and libcurl 8.7.1, and its cells read the same way.

## What differs from macOS

**The reference's cell differs in 12 rows of 443.** One is the version
libcurl announces (266). The rest:

| rows | setting | the reference on macOS | on Linux |
|---|---|---|---|
| 54, 138, 334 to 337 | a `no_proxy` entry whose number has a leading zero: `127.000.0.1`, `127.0.0.01`, `0127.0.0.1` | an address: the proxy is bypassed | not an address: through the proxy |
| 135 | host `127.0.0.10`, `no_proxy=127.0.0.010` | bypassed | through the proxy |
| 140 | host `127.0.0.010`, `no_proxy=127.0.0.8` | bypassed; nothing answers there, and it waits | bypassed; refused at once |
| 242 | a host that is not ASCII, through a proxy | asks the proxy | refuses the URL |
| 304, 305 | `[00000::1]`, `[::ffff:1.2.3.04]` | fetches | refuses the URL |

In 242, 304 and 305 Bund2 refuses on both systems. Rows 54, 135, 138 and
334 to 337 are D130.

## What differs between the two binaries, on Linux

**57 rows.** In 386 the two agree, the tunnel aside. In 51 Bund2 refuses
and asks nobody and in 5 it asks the same listener and evaluates nothing,
each under the decision the macOS table names for that row. Row 174 is
D125's file. **In no row does Bund2 ask a listener the reference does not
ask.**

At `bbd50c1` that was false of seven: 54, 135, 138 and 334 to 337. The
reference went through the proxy and Bund2 went directly, to the origin or,
in 135, to an address where nothing listens, because Bund2 read a leading
zero as macOS reads it. D130 rules that it reads one as the system it runs
on does, `17e623a` makes it so, and this run is of that commit: in all
seven Bund2 now asks the proxy, and no other cell of the 443 changed
between the two runs.

| # | setting | bund | bund2 |
|---|---|---|---|
| 1 | none | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 2 | http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 3 | HTTP_PROXY | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 4 | https_proxy | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 5 | HTTPS_PROXY | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 6 | all_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 7 | ALL_PROXY | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 8 | http_proxy=p1 all_proxy=p2 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 9 | http_proxy=p1 ALL_PROXY=p2 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 10 | all_proxy=p1 ALL_PROXY=p2 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 11 | http_proxy='' all_proxy=p2 | p2 · p2:GET http://127.0.0.1:O/lib.bund | p2 · p2:CONNECT 127.0.0.1:O, p2:GET /lib.bund |
| 12 | http_proxy no scheme | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 13 | http_proxy garbage, all_proxy=p2 | fail · - | fail · - |
| 14 | unresolvable host via http_proxy | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:CONNECT x.bund2.invalid:O, p1:GET /lib.bund |
| 15 | no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 16 | NO_PROXY=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 17 | no_proxy=other NO_PROXY=127.0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 18 | no_proxy=127.0.0.1 NO_PROXY=other | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 19 | no_proxy='' NO_PROXY=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 20 | no_proxy=* | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 21 | no_proxy=a,* | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 22 | no_proxy=a, 127.0.0.1 (space) | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 23 | no_proxy=a 127.0.0.1 (space only) | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 24 | no_proxy=127.0.0.0/8 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 25 | no_proxy=127.0.0.1:OPORT | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 26 | no_proxy=127.0.0 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 27 | no_proxy=0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 28 | host x.bund2.invalid no_proxy=x.bund2.invalid | fail · - | fail · - |
| 29 | … no_proxy=X.BUND2.INVALID | fail · - | fail · - |
| 30 | … no_proxy=bund2.invalid | fail · - | fail · - |
| 31 | … no_proxy=.bund2.invalid | fail · - | fail · - |
| 32 | … no_proxy=*.bund2.invalid | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:CONNECT x.bund2.invalid:O, p1:GET /lib.bund |
| 33 | … no_proxy=d2.invalid | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:CONNECT x.bund2.invalid:O, p1:GET /lib.bund |
| 34 | … no_proxy=invalid | fail · - | fail · - |
| 35 | … no_proxy=x.bund2.invalid. | fail · - | fail · - |
| 36 | … no_proxy=y.x.bund2.invalid | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:CONNECT x.bund2.invalid:O, p1:GET /lib.bund |
| 37 | localhost no_proxy=LOCALHOST | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 38 | all_proxy='' ALL_PROXY=p2 | p2 · p2:GET http://127.0.0.1:O/lib.bund | p2 · p2:CONNECT 127.0.0.1:O, p2:GET /lib.bund |
| 39 | http_proxy=socks5://p1 | fail · p1:SOCKS5 | fail · - |
| 40 | http_proxy=https://p1 | fail · p1:TLS | fail · - |
| 41 | http_proxy=ftp://p1 | fail · - | fail · - |
| 42 | http_proxy=user:pw@p1 | p1 · p1:GET http://127.0.0.1:O/lib.bund [proxy-auth u:p] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth u:p], p1:GET /lib.bund |
| 43 | no_proxy=' * ' | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 44 | no_proxy=127.0.0.0/0 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 45 | no_proxy=127.0.0.1/0 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 46 | no_proxy=127.0.0.1/32 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 47 | no_proxy=127.0.0.1/33 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 48 | no_proxy=127.9.9.9/8 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 49 | no_proxy=126.0.0.0/7 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 50 | no_proxy=128.0.0.0/7 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 51 | no_proxy=127.0.0.1/x | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 52 | no_proxy=.127.0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 53 | no_proxy=127.0.0.1. | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 54 | no_proxy=127.000.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 55 | host [::1] no proxy vars | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 56 | host [::1] http_proxy | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 57 | host [::1] no_proxy=::1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 58 | host [::1] no_proxy=[::1] | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 59 | host [::1] no_proxy=::/64 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 60 | host [::1] no_proxy=0:0:0:0:0:0:0:1 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 61 | host [::1] no_proxy=::1/128 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 62 | host [::1] no_proxy=::1/0 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 63 | host [::1] no_proxy=::1/64 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 64 | host [::1] no_proxy=::1/127 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 65 | host [::1] no_proxy=::0/127 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 66 | host [::1] no_proxy=::/8 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 67 | host [::1] no_proxy=0::1 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:CONNECT [::1]:O, p1:GET /lib.bund |
| 68 | host [0:0:0:0:0:0:0:1] no_proxy=::1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 69 | host x.bund2.invalid. no_proxy=bund2.invalid | fail · - | fail · - |
| 70 | … no_proxy=..bund2.invalid | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:CONNECT x.bund2.invalid:O, p1:GET /lib.bund |
| 71 | … no_proxy=bund2.invalid.. | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:CONNECT x.bund2.invalid:O, p1:GET /lib.bund |
| 72 | … no_proxy=. (dot alone) | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:CONNECT x.bund2.invalid:O, p1:GET /lib.bund |
| 73 | user@host no_proxy=127.0.0.1 | origin · origin:GET /lib.bund [auth u:p] | origin · origin:GET /lib.bund [auth u:p] |
| 74 | proxy 127.0.0.1 | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:CONNECT 127.0.0.1:O, p1080:GET /lib.bund |
| 75 | proxy http://127.0.0.1 | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:CONNECT 127.0.0.1:O, p1080:GET /lib.bund |
| 76 | proxy localhost | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:CONNECT 127.0.0.1:O, p1080:GET /lib.bund |
| 77 | proxy http://localhost | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:CONNECT 127.0.0.1:O, p1080:GET /lib.bund |
| 78 | proxy http://127.0.0.1/ | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:CONNECT 127.0.0.1:O, p1080:GET /lib.bund |
| 79 | proxy u:p@127.0.0.1 | p1080 · p1080:GET http://127.0.0.1:O/lib.bund [proxy-auth u:p] | p1080 · p1080:CONNECT 127.0.0.1:O [proxy-auth u:p], p1080:GET /lib.bund |
| 80 | proxy 127.0.0.1: | fail · - | fail · - |
| 81 | proxy http://127.0.0.1:/ | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:CONNECT 127.0.0.1:O, p1080:GET /lib.bund |
| 82 | proxy HTTP://127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 83 | proxy http://127.0.0.1:P1/path | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 84 | proxy ' http://127.0.0.1:P1' | fail · - | fail · - |
| 85 | proxy 'http://127.0.0.1:P1 ' | fail · - | fail · - |
| 86 | proxy http://127.0.0.1:P1?q | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 87 | proxy http:127.0.0.1:P1 | fail · - | fail · - |
| 88 | proxy //127.0.0.1:P1 | fail · - | fail · - |
| 89 | proxy http://127.1:P1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 90 | proxy http://127.0.0.1:0 | fail · - | fail · - |
| 91 | proxy http://127.0.0.1:99999 | fail · - | fail · - |
| 92 | proxy http://127.0.0.1:P1x | fail · - | fail · - |
| 93 | proxy socks5h://127.0.0.1 | fail · p1080:SOCKS5 | fail · - |
| 94 | host 127.0.0.1, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 95 | host 127.1, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 96 | host 2130706433, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 97 | host 0x7f.0.0.1, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 98 | host 0177.0.0.1, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 99 | host 127.0.1, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 100 | host 0x7f000001, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 101 | host 017700000001, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 102 | host 127.0.0.01, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 103 | host 127.0.0.0x1, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 104 | host 0X7F.0.0.1, no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 105 | host 127.0.0.1., no_proxy=127.0.0.1 | fail · - | fail · - |
| 106 | host 127.0.0.09, no_proxy=127.0.0.1 | p1 · p1:GET http://127.0.0.09:O/lib.bund | p1 · p1:CONNECT 127.0.0.09:O, p1:GET /lib.bund |
| 107 | host 127.0.0.256, no_proxy=127.0.0.1 | p1 · p1:GET http://127.0.0.256:O/lib.bund | p1 · p1:CONNECT 127.0.0.256:O, p1:GET /lib.bund |
| 108 | host 127.0.65536, no_proxy=127.0.0.1 | p1 · p1:GET http://127.0.65536:O/lib.bund | p1 · p1:CONNECT 127.0.65536:O, p1:GET /lib.bund |
| 109 | host 127.0.0.1.1, no_proxy=127.0.0.1 | p1 · p1:GET http://127.0.0.1.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1.1:O, p1:GET /lib.bund |
| 110 | host 1.2.3.4.5, no_proxy=127.0.0.1 | p1 · p1:GET http://1.2.3.4.5:O/lib.bund | p1 · p1:CONNECT 1.2.3.4.5:O, p1:GET /lib.bund |
| 111 | host 127..1, no_proxy=127.0.0.1 | p1 · p1:GET http://127..1:O/lib.bund | p1 · p1:CONNECT 127..1:O, p1:GET /lib.bund |
| 112 | host 0, no_proxy=127.0.0.1 | p1 · p1:GET http://0.0.0.0:O/lib.bund | p1 · p1:CONNECT 0.0.0.0:O, p1:GET /lib.bund |
| 113 | host localhost, no_proxy=127.0.0.1 | p1 · p1:GET http://localhost:O/lib.bund | p1 · p1:CONNECT localhost:O, p1:GET /lib.bund |
| 114 | host LocalHost, no_proxy=127.0.0.1 | p1 · p1:GET http://LocalHost:O/lib.bund | p1 · p1:CONNECT LocalHost:O, p1:GET /lib.bund |
| 115 | host 127.1, no proxy at all | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 116 | host 127.0.0.1, no_proxy=127.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 117 | host 127.0.0.1, no_proxy=0x7f.0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 118 | host 127.0.0.1, no_proxy=2130706433 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 119 | host 127.0.0.1, no_proxy=0177.0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 120 | host 127.0.0.1, no_proxy=127.0.0.010/32 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 121 | url HTTP:// | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 122 | url HTTP:// via http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 123 | url Http:// | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 124 | url Http:// via http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 125 | url hTTp:// | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 126 | url hTTp:// via http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 127 | url file:///abs/lib%20file.bund | file · - | file · - |
| 128 | url FILE:///abs/lib%20file.bund | file · - | file · - |
| 129 | url File:///abs/lib%20file.bund | file · - | file · - |
| 130 | url file://localhost/abs/lib%20file.bund | file · - | file · - |
| 131 | url file://LOCALHOST/abs/lib%20file.bund | file · - | file · - |
| 132 | url FILE://LocalHost/abs/lib%20file.bund | file · - | file · - |
| 133 | url file://127.0.0.1/abs/lib%20file.bund | file · - | file · - |
| 134 | url file://abs/lib.bund | fail · - | fail · - |
| 135 | host 127.0.0.10, no_proxy=127.0.0.010 | p1 · p1:GET http://127.0.0.10:O/lib.bund | p1 · p1:CONNECT 127.0.0.10:O, p1:GET /lib.bund |
| 136 | host 127.0.0.8, no_proxy=127.0.0.010 | p1 · p1:GET http://127.0.0.8:O/lib.bund | p1 · p1:CONNECT 127.0.0.8:O, p1:GET /lib.bund |
| 137 | host 127.0.0.10, no_proxy=127.0.0.012 | p1 · p1:GET http://127.0.0.10:O/lib.bund | p1 · p1:CONNECT 127.0.0.10:O, p1:GET /lib.bund |
| 138 | host 127.0.0.1, no_proxy=127.0.0.01 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /lib.bund |
| 139 | host 127.0.0.1, no_proxy=127.0.0.1/8x | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 140 | host 127.0.0.010, no_proxy=127.0.0.8 | fail · - | fail · - |
| 141 | host 127.0.0.010, no_proxy=127.0.0.10 | p1 · p1:GET http://127.0.0.8:O/lib.bund | p1 · p1:CONNECT 127.0.0.8:O, p1:GET /lib.bund |
| 142 | host 0x7f.1, no_proxy=127.0.0.0/8 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 143 | host 127.0.0.1, no_proxy=127.0.0.1, | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 144 | host 0, no_proxy=0.0.0.0 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 145 | url http:/127.0.0.1:OPORT/lib.bund | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 146 | url http:///127.0.0.1:OPORT/lib.bund | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 147 | url http:////127.0.0.1:OPORT/lib.bund | fail · - | fail · - |
| 148 | url http:127.0.0.1:OPORT/lib.bund | fail · - | fail · - |
| 149 | url http:\\127.0.0.1:OPORT/lib.bund | fail · - | fail · - |
| 150 | url  http://127.0.0.1:OPORT/lib.bund | fail · - | fail · - |
| 151 | url http://127.0.0.1:OPORT/lib.bund  | fail · - | fail · - |
| 152 | url http://127.0.0.1:OPORT/a b | fail · - | fail · - |
| 153 | url http://127.0.0.1:OPORT | origin · origin:GET / | origin · origin:GET / |
| 154 | url http://127.0.0.1:OPORT?x | origin · origin:GET /?x | origin · origin:GET /?x |
| 155 | url http://127.0.0.1:OPORT#f | origin · origin:GET / | origin · origin:GET / |
| 156 | url http://u:p@127.0.0.1:OPORT/x | origin · origin:GET /x [auth u:p] | origin · origin:GET /x [auth u:p] |
| 157 | url http://127.0.0.1:0OPORT/x | origin · origin:GET /x | origin · origin:GET /x |
| 158 | url 127.0.0.1:OPORT/lib.bund | origin · origin:GET /lib.bund | fail · - |
| 159 | url //127.0.0.1:OPORT/lib.bund | fail · - | fail · - |
| 160 | url HTTPS://127.0.0.1:OPORT/x | fail · origin:TLS | fail · - |
| 161 | url ftp://127.0.0.1:OPORT/x | fail · origin:silent | fail · - |
| 162 | url http://127.0.0.1:OPORT/%41?b=%20 | origin · origin:GET /%41?b=%20 | origin · origin:GET /%41?b=%20 |
| 163 | url file:/abs/lib%20file.bund | file · - | file · - |
| 164 | url file:////abs/lib%20file.bund | file · - | file · - |
| 165 | url file://localhost | fail · - | fail · - |
| 166 | url file://Localhost./abs/lib%20file.bund | fail · - | fail · - |
| 167 | url file://127.1/abs/lib%20file.bund | fail · - | fail · - |
| 168 | url file://[::1]/abs/lib%20file.bund | fail · - | fail · - |
| 169 | url file://localhost:80/abs/lib%20file.bund | fail · - | fail · - |
| 170 | url file://u@localhost/abs/lib%20file.bund | fail · - | fail · - |
| 171 | url file:///abs/lib%20file.bund?x | file · - | file · - |
| 172 | url file:///abs/lib%20file.bund#f | file · - | file · - |
| 173 | url file:///abs/lib%20file.bund%00 | fail · - | fail · - |
| 174 | url file:///abs/lib file.bund | fail · - | file · - |
| 175 | url file:rel/lib.bund | fail · - | fail · - |
| 176 | path /a/../lib.bund | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 177 | path /a/./b | origin · origin:GET /a/b | origin · origin:GET /a/b |
| 178 | path /a/b/.. | origin · origin:GET /a/ | origin · origin:GET /a/ |
| 179 | path /a/b/. | origin · origin:GET /a/b/ | origin · origin:GET /a/b/ |
| 180 | path /../x | origin · origin:GET /x | origin · origin:GET /x |
| 181 | path /a/../../x | origin · origin:GET /x | origin · origin:GET /x |
| 182 | path /a//b | origin · origin:GET /a//b | origin · origin:GET /a//b |
| 183 | path /./ | origin · origin:GET / | origin · origin:GET / |
| 184 | path /.. | origin · origin:GET / | origin · origin:GET / |
| 185 | path /a/%2e%2e/b | origin · origin:GET /a/%2e%2e/b | origin · origin:GET /a/%2e%2e/b |
| 186 | path /a/%2E/b | origin · origin:GET /a/%2E/b | origin · origin:GET /a/%2E/b |
| 187 | path /a/.%2e/b | origin · origin:GET /a/.%2e/b | origin · origin:GET /a/.%2e/b |
| 188 | path /a/..;x/b | origin · origin:GET /a/..;x/b | origin · origin:GET /a/..;x/b |
| 189 | path /a/... | origin · origin:GET /a/... | origin · origin:GET /a/... |
| 190 | path /a/..b/c | origin · origin:GET /a/..b/c | origin · origin:GET /a/..b/c |
| 191 | path /.a/b | origin · origin:GET /.a/b | origin · origin:GET /.a/b |
| 192 | path /a/../b?x=/../y | origin · origin:GET /b?x=/../y | origin · origin:GET /b?x=/../y |
| 193 | path /a/../b#f/../z | origin · origin:GET /b | origin · origin:GET /b |
| 194 | path /é.bund | origin · origin:GET /%c3%a9.bund | origin · origin:GET /%c3%a9.bund |
| 195 | path /a?q=é | origin · origin:GET /a?q=Ã© | origin · origin:GET /a?q=Ã© |
| 196 | path /%C3%A9 | origin · origin:GET /%C3%A9 | origin · origin:GET /%C3%A9 |
| 197 | path /%zz | origin · origin:GET /%zz | origin · origin:GET /%zz |
| 198 | path /a\\b | origin · origin:GET /a\\b | origin · origin:GET /a\\b |
| 199 | path  | origin · origin:GET / | origin · origin:GET / |
| 200 | path ?x | origin · origin:GET /?x | origin · origin:GET /?x |
| 201 | path /x#frag | origin · origin:GET /x | origin · origin:GET /x |
| 202 | path /a/b/../../../c | origin · origin:GET /c | origin · origin:GET /c |
| 203 | path /a/./../b/ | origin · origin:GET /b/ | origin · origin:GET /b/ |
| 204 | path /~x/$y,z;w=1@:!*'() | origin · origin:GET /~x/$y,z;w=1@:!*'() | origin · origin:GET /~x/$y,z;w=1@:!*'() |
| 205 | path /a<tab>b | fail · - | fail · - |
| 206 | path /<nbsp> | origin · origin:GET /%c2%a0 | origin · origin:GET /%c2%a0 |
| 207 | path /x?é#é | origin · origin:GET /x?Ã© | origin · origin:GET /x?Ã© |
| 208 | path /a<b | origin · origin:GET /a<b | fail · - |
| 209 | path /a>b | origin · origin:GET /a>b | fail · - |
| 210 | path /a`b | origin · origin:GET /a`b | fail · - |
| 211 | path /a?< | origin · origin:GET /a?< | fail · - |
| 212 | path /a^b&#124;c{d}e[f] | origin · origin:GET /a^b|c{d}e[f] | origin · origin:GET /a^b|c{d}e[f] |
| 213 | url port :65536 | fail · - | fail · - |
| 214 | url port :99999 | fail · - | fail · - |
| 215 | url port :65616 | fail · - | fail · - |
| 216 | url port :4294967376 | fail · - | fail · - |
| 217 | url port :Ox | fail · - | fail · - |
| 218 | url port :0 | fail · - | fail · - |
| 219 | url port : | p80 · p80:GET /x | p80 · p80:GET /x |
| 220 | url port :00O | origin · origin:GET /x | origin · origin:GET /x |
| 221 | url port :+O | fail · - | fail · - |
| 222 | url port :-1 | fail · - | fail · - |
| 223 | url port : O | fail · - | fail · - |
| 224 | url port :O:O | fail · - | fail · - |
| 225 | url port :65536, host localhost | fail · - | fail · - |
| 226 | url port :65536, via http_proxy | fail · - | fail · - |
| 227 | url http://a@b@127.0.0.1:O/x | fail · - | fail · - |
| 228 | url http://a:b:c@127.0.0.1:O/x | origin · origin:GET /x [auth a:b:c] | origin · origin:GET /x [auth a:b:c] |
| 229 | url http://@127.0.0.1:O/x | origin · origin:GET /x [auth :] | origin · origin:GET /x [auth :] |
| 230 | url http://:@127.0.0.1:O/x | origin · origin:GET /x [auth :] | origin · origin:GET /x [auth :] |
| 231 | url http://a%40b@127.0.0.1:O/x | origin · origin:GET /x [auth a@b:] | origin · origin:GET /x [auth a@b:] |
| 232 | url http://u s@127.0.0.1:O/x | fail · - | fail · - |
| 233 | url http://[::ffff:127.0.0.1]:O/x | origin · origin:GET /x | origin · origin:GET /x |
| 234 | url http://127.0.0.1.:O/x | fail · - | fail · - |
| 235 | url http://LOCALHOST:O/x | origin · origin:GET /x | origin · origin:GET /x |
| 236 | url http://local_host:O/x | fail · - | fail · - |
| 237 | url http://-x:O/x | fail · - | fail · - |
| 238 | url http://:O/x | fail · - | fail · - |
| 239 | url http://127.0.0.%31:O/x | origin · origin:GET /x | fail · - |
| 240 | url http://%6cocalhost:O/x | origin · origin:GET /x | fail · - |
| 241 | url http://[::1%25lo0]:O/x | origin · origin:GET /x | fail · - |
| 242 | url host bücher.invalid, via http_proxy | fail · - | fail · - |
| 243 | proxy 127.0.0.1:P1/ | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 244 | proxy 127.0.0.1:P1/x | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 245 | proxy 127.0.0.1:P1?q | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 246 | proxy localhost:P1/x | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 247 | proxy 127.0.0.1/ | p1080 · p1080:GET http://127.0.0.1:O/x | p1080 · p1080:CONNECT 127.0.0.1:O, p1080:GET /x |
| 248 | proxy 127.0.0.1/x | p1080 · p1080:GET http://127.0.0.1:O/x | p1080 · p1080:CONNECT 127.0.0.1:O, p1080:GET /x |
| 249 | proxy http://127.0.0.1:P1/x?q#f | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 250 | proxy http://127.0.0.1:65536 | fail · - | fail · - |
| 251 | proxy http://127.0.0.1:99999 | fail · - | fail · - |
| 252 | proxy http://127.0.0.1:P1x | fail · - | fail · - |
| 253 | proxy 127.0.0.1:99999 | fail · - | fail · - |
| 254 | proxy http://127.0.0.1:0P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 255 | proxy http://a@b@127.0.0.1:P1 | fail · - | fail · - |
| 256 | proxy http://u:p@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth u:p], p1:GET /x |
| 257 | proxy http://127.0.0.1:P1:1 | fail · - | fail · - |
| 258 | proxy http://LOCALHOST:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 259 | proxy http://0x7f.1:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 260 | proxy http://[::ffff:127.0.0.1]:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 261 | proxy http://127.0.0.%31:P1 | p1 · p1:GET http://127.0.0.1:O/x | fail · - |
| 262 | proxy https://127.0.0.1 | fail · - | fail · - |
| 263 | proxy socks5://127.0.0.1 | fail · p1080:SOCKS5 | fail · - |
| 264 | scheme gopher:// | origin · origin:other /lib.bund\r\n | fail · - |
| 265 | scheme gopher:// with http_proxy | origin · origin:other /lib.bund\r\n | fail · - |
| 266 | scheme dict:// | origin · origin:other CLIENT libcurl 8.5.0\r\nDE | fail · - |
| 267 | scheme ftp:// | fail · origin:silent | fail · - |
| 268 | scheme ftp:// with ftp_proxy | p1 · p1:GET ftp://127.0.0.1:O/lib.bund | fail · - |
| 269 | scheme ftp:// with all_proxy | p1 · p1:GET ftp://127.0.0.1:O/lib.bund | fail · - |
| 270 | scheme ftp:// with http_proxy | fail · origin:silent | fail · - |
| 271 | scheme telnet:// | origin · origin:silent | fail · - |
| 272 | scheme https:// | fail · origin:TLS | fail · - |
| 273 | scheme https:// with https_proxy | fail · p1:CONNECT 127.0.0.1:O, p1:TLS | fail · - |
| 274 | scheme ws:// | fail · - | fail · - |
| 275 | scheme smb:// | fail · origin:silent | fail · - |
| 276 | scheme tftp:// | timeout · - | fail · - |
| 277 | scheme no scheme, host:port/path | origin · origin:GET /lib.bund | fail · - |
| 278 | scheme no scheme, with http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | fail · - |
| 279 | host a!b.invalid, via http_proxy | fail · - | fail · - |
| 280 | host a$b.invalid, via http_proxy | fail · - | fail · - |
| 281 | host a&b.invalid, via http_proxy | fail · - | fail · - |
| 282 | host a'b.invalid, via http_proxy | fail · - | fail · - |
| 283 | host a(b.invalid, via http_proxy | fail · - | fail · - |
| 284 | host a)b.invalid, via http_proxy | fail · - | fail · - |
| 285 | host a*b.invalid, via http_proxy | fail · - | fail · - |
| 286 | host a+b.invalid, via http_proxy | fail · - | fail · - |
| 287 | host a,b.invalid, via http_proxy | fail · - | fail · - |
| 288 | host a;b.invalid, via http_proxy | fail · - | fail · - |
| 289 | host a=b.invalid, via http_proxy | fail · - | fail · - |
| 290 | host a&#124;b.invalid, via http_proxy | p1 · p1:GET http://a|b.invalid:O/x | fail · - |
| 291 | host a~b.invalid, via http_proxy | p1 · p1:GET http://a~b.invalid:O/x | p1 · p1:CONNECT a~b.invalid:O, p1:GET /x |
| 292 | host a_b.invalid, via http_proxy | p1 · p1:GET http://a_b.invalid:O/x | p1 · p1:CONNECT a_b.invalid:O, p1:GET /x |
| 293 | host a-b.invalid, via http_proxy | p1 · p1:GET http://a-b.invalid:O/x | p1 · p1:CONNECT a-b.invalid:O, p1:GET /x |
| 294 | host a!b.invalid, no proxy | fail · - | fail · - |
| 295 | host a~b.invalid, no proxy | fail · - | fail · - |
| 296 | host 127.0.0.1!, via http_proxy | fail · - | fail · - |
| 297 | host 127.0.0.1,1, via http_proxy | fail · - | fail · - |
| 298 | host [zz], via http_proxy | fail · - | fail · - |
| 299 | host [::g], via http_proxy | fail · - | fail · - |
| 300 | host [1.2.3.4], via http_proxy | fail · - | fail · - |
| 301 | host [], via http_proxy | fail · - | fail · - |
| 302 | host [::1]x, via http_proxy | fail · - | fail · - |
| 303 | host [:::1], via http_proxy | fail · - | fail · - |
| 304 | host [00000::1], via http_proxy | fail · - | fail · - |
| 305 | host [::ffff:1.2.3.04], via http_proxy | fail · - | fail · - |
| 306 | host [0:0:0:0:0:0:0:1], no proxy | origin · origin:GET /x [host [::1]:O] | origin · origin:GET /x [host [::1]:O] |
| 307 | host [::0001], no proxy | origin · origin:GET /x [host [::1]:O] | origin · origin:GET /x [host [::1]:O] |
| 308 | host [0::0:1], no proxy | origin · origin:GET /x [host [::1]:O] | origin · origin:GET /x [host [::1]:O] |
| 309 | host [::FFFF:127.0.0.1], no proxy | origin · origin:GET /x [host [::FFFF:127.0.0.1]:O] | origin · origin:GET /x [host [::FFFF:127.0.0.1]:O] |
| 310 | host [::ffff:7f00:1], no proxy | origin · origin:GET /x [host [::ffff:7f00:1]:O] | origin · origin:GET /x [host [::ffff:7f00:1]:O] |
| 311 | host [0:0:0:0:0:0:0:1], via http_proxy | p1 · p1:GET http://[::1]:O/x | p1 · p1:CONNECT [::1]:O, p1:GET /x |
| 312 | host [0:0:0:0:0:0:0:2], via http_proxy | p1 · p1:GET http://[::2]:O/x | p1 · p1:CONNECT [::2]:O, p1:GET /x |
| 313 | host [::2], via http_proxy | p1 · p1:GET http://[::2]:O/x | p1 · p1:CONNECT [::2]:O, p1:GET /x |
| 314 | host [::ffff:7f00:1], via http_proxy | p1 · p1:GET http://[::ffff:7f00:1]:O/x | p1 · p1:CONNECT [::ffff:7f00:1]:O, p1:GET /x |
| 315 | host [::FFFF:127.0.0.1], via http_proxy | p1 · p1:GET http://[::FFFF:127.0.0.1]:O/x | p1 · p1:CONNECT [::FFFF:127.0.0.1]:O, p1:GET /x |
| 316 | host [0:0:0:0:0:0:0:A], via http_proxy | p1 · p1:GET http://[::a]:O/x | p1 · p1:CONNECT [::a]:O, p1:GET /x |
| 317 | host [::A], via http_proxy | p1 · p1:GET http://[::A]:O/x | p1 · p1:CONNECT [::A]:O, p1:GET /x |
| 318 | host [1:0:0:2:0:0:0:3], via http_proxy | p1 · p1:GET http://[1:0:0:2::3]:O/x | p1 · p1:CONNECT [1:0:0:2::3]:O, p1:GET /x |
| 319 | host [1::2:0:0:0:3], via http_proxy | p1 · p1:GET http://[1:0:0:2::3]:O/x | p1 · p1:CONNECT [1:0:0:2::3]:O, p1:GET /x |
| 320 | host [0:0:1::], via http_proxy | p1 · p1:GET http://[0:0:1::]:O/x | p1 · p1:CONNECT [0:0:1::]:O, p1:GET /x |
| 321 | host [::1.2.3.4], via http_proxy | p1 · p1:GET http://[::1.2.3.4]:O/x | p1 · p1:CONNECT [::1.2.3.4]:O, p1:GET /x |
| 322 | host [0:0:0:0:0:0:102:304], via http_proxy | p1 · p1:GET http://[::1.2.3.4]:O/x | p1 · p1:CONNECT [::1.2.3.4]:O, p1:GET /x |
| 323 | host [::ffff:7f00:1], no_proxy=::ffff:7f00:1 | origin · origin:GET /x | origin · origin:GET /x |
| 324 | host [::ffff:7f00:1], no_proxy=::ffff:127.0.0.1 | p1 · p1:GET http://[::ffff:7f00:1]:O/x | p1 · p1:CONNECT [::ffff:7f00:1]:O, p1:GET /x |
| 325 | host [::FFFF:127.0.0.1], no_proxy=::ffff:127.0.0.1 | origin · origin:GET /x | origin · origin:GET /x |
| 326 | host [::FFFF:127.0.0.1], no_proxy=0.1 | origin · origin:GET /x | origin · origin:GET /x |
| 327 | host [::FFFF:127.0.0.1], no_proxy=127.0.0.1 | p1 · p1:GET http://[::FFFF:127.0.0.1]:O/x | p1 · p1:CONNECT [::FFFF:127.0.0.1]:O, p1:GET /x |
| 328 | host [0:0:0:0:0:0:0:1], no_proxy=0:0:0:0:0:0:0:1 | p1 · p1:GET http://[::1]:O/x | p1 · p1:CONNECT [::1]:O, p1:GET /x |
| 329 | host [::0001], no_proxy=::1 | origin · origin:GET /x | origin · origin:GET /x |
| 330 | host [::0001], no_proxy=::0001 | p1 · p1:GET http://[::1]:O/x | p1 · p1:CONNECT [::1]:O, p1:GET /x |
| 331 | host [0:0:0:0:0:0:0:2], no_proxy=::2 | fail · - | fail · - |
| 332 | host [0:0:0:0:0:0:0:2], no_proxy=::0.0.0.2 | p1 · p1:GET http://[::2]:O/x | p1 · p1:CONNECT [::2]:O, p1:GET /x |
| 333 | host [::2], no_proxy=::0.0.0.2 | p1 · p1:GET http://[::2]:O/x | p1 · p1:CONNECT [::2]:O, p1:GET /x |
| 334 | no_proxy=0127.0.0.1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 335 | no_proxy=00127.0.0.1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 336 | no_proxy=127.0.0.0001 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 337 | no_proxy=127.0.0.0000000000000000000000000000001 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 338 | no_proxy=127.0.0.9/+8 | origin · origin:GET /x | origin · origin:GET /x |
| 339 | no_proxy=127.0.0.9/4294967304 | origin · origin:GET /x | origin · origin:GET /x |
| 340 | no_proxy=127.0.0.1/+33 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 341 | no_proxy=127.0.0.9/-8 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 342 | no_proxy=127.0.0.1/-0 | origin · origin:GET /x | origin · origin:GET /x |
| 343 | no_proxy=127.0.0.9/-0 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 344 | no_proxy=127.0.0.9/08 | origin · origin:GET /x | origin · origin:GET /x |
| 345 | no_proxy=127.0.0.1/4294967296 | origin · origin:GET /x | origin · origin:GET /x |
| 346 | no_proxy=127.0.0.9/4294967296 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 347 | no_proxy=127.0.0.9/4294967328 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 348 | no_proxy=127.0.0.1/99999999999999999999 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 349 | no_proxy=127.0.0.9/99999999999999999999 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 350 | no_proxy=127.0.0.1/-1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 351 | no_proxy=127.0.0.1/+ | origin · origin:GET /x | origin · origin:GET /x |
| 352 | no_proxy=127.0.0.1/ | origin · origin:GET /x | origin · origin:GET /x |
| 353 | no_proxy=127.0.0.9/8/1 | origin · origin:GET /x | origin · origin:GET /x |
| 354 | no_proxy=127.0.0.1/+0x | origin · origin:GET /x | origin · origin:GET /x |
| 355 | no_proxy=127.0.0.9/++8 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 356 | no_proxy=127.0.0.9/+-8 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 357 | no_proxy=127.0.0.9/-4294967288 | origin · origin:GET /x | origin · origin:GET /x |
| 358 | no_proxy=127.0.0.1/-99999999999999999999 | origin · origin:GET /x | origin · origin:GET /x |
| 359 | no_proxy=127.0.0.9/<newline>8 | origin · origin:GET /x | origin · origin:GET /x |
| 360 | no_proxy=0300.0.0.1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 361 | url user part `u:p@` | origin · origin:GET /x [auth u:p] | origin · origin:GET /x [auth u:p] |
| 362 | url user part `a%00b@` | fail · - | fail · - |
| 363 | url user part `u:%00@` | fail · - | fail · - |
| 364 | url user part `a%01b@` | origin · origin:GET /x [auth a\x01b:] | origin · origin:GET /x [auth a\x01b:] |
| 365 | url user part `a%09b@` | origin · origin:GET /x [auth a\tb:] | origin · origin:GET /x [auth a\tb:] |
| 366 | url user part `a%0ab@` | origin · origin:GET /x [auth a\nb:] | origin · origin:GET /x [auth a\nb:] |
| 367 | url user part `a%0db@` | origin · origin:GET /x [auth a\rb:] | origin · origin:GET /x [auth a\rb:] |
| 368 | url user part `a%1fb@` | origin · origin:GET /x [auth a\x1fb:] | origin · origin:GET /x [auth a\x1fb:] |
| 369 | url user part `u:%1f@` | origin · origin:GET /x [auth u:\x1f] | origin · origin:GET /x [auth u:\x1f] |
| 370 | url user part `a%20b@` | origin · origin:GET /x [auth a b:] | origin · origin:GET /x [auth a b:] |
| 371 | url user part `a%7fb@` | origin · origin:GET /x [auth a\x7fb:] | origin · origin:GET /x [auth a\x7fb:] |
| 372 | url user part `a%80b@` | origin · origin:GET /x [auth a\x80b:] | origin · origin:GET /x [auth a\x80b:] |
| 373 | url user part `a%ffb@` | origin · origin:GET /x [auth a\xffb:] | origin · origin:GET /x [auth a\xffb:] |
| 374 | url user part `a%40b:c%3Ad@` | origin · origin:GET /x [auth a@b:c:d] | origin · origin:GET /x [auth a@b:c:d] |
| 375 | url user part `u%20s@` | origin · origin:GET /x [auth u s:] | origin · origin:GET /x [auth u s:] |
| 376 | url user part `%zz@` | origin · origin:GET /x [auth %zz:] | origin · origin:GET /x [auth %zz:] |
| 377 | url user part `a%@` | origin · origin:GET /x [auth a%:] | origin · origin:GET /x [auth a%:] |
| 378 | url user part `a%4@` | origin · origin:GET /x [auth a%4:] | origin · origin:GET /x [auth a%4:] |
| 379 | url user part `a<b@` | origin · origin:GET /x [auth a<b:] | origin · origin:GET /x [auth a<b:] |
| 380 | url user part `a&#124;b@` | origin · origin:GET /x [auth a|b:] | origin · origin:GET /x [auth a|b:] |
| 381 | url user part `é@` | origin · origin:GET /x [auth \xc3\xa9:] | origin · origin:GET /x [auth \xc3\xa9:] |
| 382 | url user part `@` | origin · origin:GET /x [auth :] | origin · origin:GET /x [auth :] |
| 383 | url user part `:@` | origin · origin:GET /x [auth :] | origin · origin:GET /x [auth :] |
| 384 | url user part `u:@` | origin · origin:GET /x [auth u:] | origin · origin:GET /x [auth u:] |
| 385 | url user part `:p@` | origin · origin:GET /x [auth :p] | origin · origin:GET /x [auth :p] |
| 386 | url user part `a:b:c@` | origin · origin:GET /x [auth a:b:c] | origin · origin:GET /x [auth a:b:c] |
| 387 | url user part `a;b@` | origin · origin:GET /x [auth a;b:] | origin · origin:GET /x [auth a;b:] |
| 388 | url user part `u:p%2@` | origin · origin:GET /x [auth u:p%2] | origin · origin:GET /x [auth u:p%2] |
| 389 | url user part `u:p@`, via http_proxy | p1 · p1:GET http://127.0.0.1:O/x [auth u:p] | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x [auth u:p] |
| 390 | url user part `a%00b@`, via http_proxy | fail · - | fail · - |
| 391 | url user part `a%40b@`, via http_proxy | p1 · p1:GET http://127.0.0.1:O/x [auth a@b:] | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x [auth a@b:] |
| 392 | proxy http://u:p@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth u:p], p1:GET /x |
| 393 | proxy http://a%00b@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 394 | proxy http://u:%00@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 395 | proxy http://a%01b@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 396 | proxy http://a%09b@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 397 | proxy http://a%0ab@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 398 | proxy http://a%0db@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 399 | proxy http://a%1fb@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 400 | proxy http://u:%1f@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 401 | proxy http://a%20b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a b:] | fail · - |
| 402 | proxy http://a%7fb@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\x7fb:] | fail · - |
| 403 | proxy http://a%80b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\x80b:] | fail · - |
| 404 | proxy http://a%ffb@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\xffb:] | fail · - |
| 405 | proxy http://a%40b:c%3Ad@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a@b:c:d] | fail · - |
| 406 | proxy http://u%20s@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u s:] | fail · - |
| 407 | proxy http://%zz@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth %zz:] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth %zz:], p1:GET /x |
| 408 | proxy http://a%@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a%:] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth a%:], p1:GET /x |
| 409 | proxy http://a%4@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a%4:] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth a%4:], p1:GET /x |
| 410 | proxy http://a<b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a<b:] | fail · - |
| 411 | proxy http://a&#124;b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a|b:] | fail · - |
| 412 | proxy http://é@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth \xc3\xa9:] | fail · - |
| 413 | proxy http://@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth :], p1:GET /x |
| 414 | proxy http://:@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth :], p1:GET /x |
| 415 | proxy http://u:@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth u:], p1:GET /x |
| 416 | proxy http://:p@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :p] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth :p], p1:GET /x |
| 417 | proxy http://a:b:c@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a:b:c] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth a:b:c], p1:GET /x |
| 418 | proxy http://a;b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a;b:] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth a;b:], p1:GET /x |
| 419 | proxy http://u:p%2@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p%2] | p1 · p1:CONNECT 127.0.0.1:O [proxy-auth u:p%2], p1:GET /x |
| 420 | proxy a%00b@127.0.0.1:P1, no scheme | origin · origin:GET /x | fail · - |
| 421 | all_proxy http://a%00b@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 422 | http_proxy http://a%00b@127.0.0.1:P1, all_proxy=p2 | origin · origin:GET /x | fail · - |
| 423 | proxy http://127.0.0.1:P1#f | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:CONNECT 127.0.0.1:O, p1:GET /x |
| 424 | url file:///abs/d/link/../t.bund | file · - | file · - |
| 425 | url file://localhost/abs/d/link/../t.bund | file · - | file · - |
| 426 | url the `file` word, /abs/d/link/../t.bund | file · - | file · - |
| 427 | url file:///abs/d/nosuch/../t.bund | file · - | file · - |
| 428 | url file:///abs/d/data.txt/../t.bund | file · - | file · - |
| 429 | url file:///abs/d/link/%2e%2e/t.bund | linked · - | linked · - |
| 430 | url file:///abs/d/real/sub/../t.bund | linked · - | linked · - |
| 431 | url file:///abs/d/link/./../t.bund | file · - | file · - |
| 432 | url file:///abs/d/./t.bund | file · - | file · - |
| 433 | url file:///abs/d/link/.. | fail · - | fail · - |
| 434 | url the `file` word, /abs/d/link/%2e%2e/t.bund | linked · - | linked · - |
| 435 | path /h126 | origin · origin:GET /h126 | origin · origin:GET /h126 |
| 436 | path /h127 | origin · origin:GET /h127 | origin · origin:GET /h127 |
| 437 | path /h128 | origin · origin:GET /h128 | origin · origin:GET /h128 |
| 438 | path /h129 | origin · origin:GET /h129 | fail · origin:GET /h129 |
| 439 | path /h130 | origin · origin:GET /h130 | fail · origin:GET /h130 |
| 440 | path /h200 | origin · origin:GET /h200 | fail · origin:GET /h200 |
| 441 | path /v2 | origin · origin:GET /v2 | fail · origin:GET /v2 |
| 442 | path /a?<> | origin · origin:GET /a?<> | fail · - |
| 443 | a proxy that refuses CONNECT and answers GET | p3 · p3:GET http://127.0.0.1:O/x | fail · p3:CONNECT 127.0.0.1:O |
