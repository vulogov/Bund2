# Where a fetch goes on Linux: the oracle and Bund2, 741 settings

Measured 2026-10-10 on `ubuntu-latest`, x86-64. The oracle is
`reference/Bund` at `21b40b0`, built out of tree by the workflow, and it
links the system's libcurl dynamically; row 266 has it announce itself as
8.5.0. Bund2 is `6c84d9e`, the commit of D131 and D132. The run is workflow
`measure-fetch`, 38026701887; `fetch.py` printed the table below exactly,
with `--port80`. It is the companion of `fetch-2026-10-09.md`, which is
macOS arm64 and libcurl 8.7.1, of the same tree and the same 741 settings
in the same rows, and its cells read the same way.

**This table is a recorded expectation since RFC-0006's criterion 14
(D136).** The workflow compares the table it prints with the one below,
cell for cell, and fails on a difference. To change a cell here is to say
the fetch changed, or the runner's libcurl did, and which decision approves
it.

**The file's name is the day of its first version; the measurement is of
2026-10-10.** It is not renamed because decisions and defects cite it by
this name.

**Two cells in row 735 are written by hand and not by that run**, both to
the same end. The run printed a long request line cut after the first
three digits of a listener's port, `http://127.0.0.1:441`, and a port
differs every run, so no later table could equal this one. `fetch.py` now
names the port before it cuts and the cells read `http://127.0.0.1:O/a`.
The macOS table's cell was rewritten the same way and a run there printed
exactly that. On Linux the first run of the gate is what confirms it.

*(This is the third version. The first was run 37994971955, of Bund2 at
`bbd50c1`, and the second run 38002480092, of `17e623a`: 443 settings each,
of the tree before D131, where `ureq` wrote Bund2's requests and read its
responses. Rows 1 to 443 here are those settings again. No cell of the
reference's changed in them between the second run and this one.)*

## What differs from macOS

**The reference's cell differs in 28 rows of 741.** One is the version
libcurl announces (266). Eleven were in the 443 and are as they were:

| rows | setting | the reference on macOS | on Linux |
|---|---|---|---|
| 54, 138, 334 to 337 | a `no_proxy` entry whose number has a leading zero: `127.000.0.1`, `127.0.0.01`, `0127.0.0.1` | an address: the proxy is bypassed | not an address: through the proxy |
| 135 | host `127.0.0.10`, `no_proxy=127.0.0.010` | bypassed | through the proxy |
| 140 | host `127.0.0.010`, `no_proxy=127.0.0.8` | bypassed; nothing answers there, and it waits | bypassed; refused at once |
| 242 | a host that is not ASCII, through a proxy | asks the proxy | refuses the URL |
| 304, 305 | `[00000::1]`, `[::ffff:1.2.3.04]` | fetches | refuses the URL |

In 242, 304 and 305 Bund2 refuses on both systems. Rows 54, 135, 138 and
334 to 337 are D130, and Bund2's cell follows the reference's in them and
in 140.

**Sixteen are new, and all are in rows 444 to 741: how a response is read,
and how long a request may be.** In every one Bund2's cell is the same on
both systems, and is the reference's on macOS:

| rows | the response, or the request | the reference on macOS, and Bund2 on both | the reference on Linux |
|---|---|---|---|
| 499, 500, 688, 693 | `chunked` named twice, in two lines or in one, and the body framed once | fails | the body |
| 621, 622, 655 | `chunked` named two, three or four times and the body framed as many | the body | the body with one layer of framing taken off: the rest of the framing is text |
| 656, 657 | named five or six times and framed as many | fails | the body with one layer taken off |
| 658, 659 | named twice and framed twice, the inner framing unfinished or with bytes after it | the body | the inner framing, as text |
| 660 | named twice and framed twice, the outer framing unfinished | the body | fails |
| 570 | the answer ends after a `Transfer-Encoding: chunked` line, before the blank line | the empty string | fails |
| 635 | the last chunk's final line ends at a carriage return | fails | the body |
| 734, 735 | a request of 1,048,575 bytes, directly and through a proxy | sent whole, and the answer fetched | fails; the listener received bytes that are not a whole request |

The first five lines are what one rule would give: libcurl 8.5.0 takes off
one layer of chunked framing however often `chunked` is named, and 8.7.1 as
many as are named, up to four. That is a reading of twelve rows and not
something either libcurl was read for.

## What differs between the two binaries, on Linux

**55 rows. In 686 the two cells are identical.**

- **37: Bund2 refuses and asks nobody**, each under the decision the macOS
  table names for that row. On macOS these are 40: in 242, 304 and 305 the
  reference refuses on Linux too.
- **2: rows 174 and 724**, D125's file whose name holds a space.
- **16: the rows of the table above.** The reference differs from itself by
  the libcurl it links and Bund2 keeps one reader, the model of 8.7.1.
  **D133 approves these as a deviation on Linux**, and F188 is the defect.
  By what a program sees: in seven Bund2 fails where the reference fetches
  (499, 500, 635, 656, 657, 688, 693); in five the two fetch different
  text (621, 622, 655, 658, 659); **in four Bund2 fetches where the
  reference fails**, the empty string in 570 and the body in 660, 734 and
  735.

**In no row does Bund2 ask a listener the reference does not ask.** In 734
and 735 both ask the same listener, and what differs is how much of the
request arrives.

| # | setting | bund | bund2 |
|---|---|---|---|
| 1 | none | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 2 | http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 3 | HTTP_PROXY | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 4 | https_proxy | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 5 | HTTPS_PROXY | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 6 | all_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 7 | ALL_PROXY | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 8 | http_proxy=p1 all_proxy=p2 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 9 | http_proxy=p1 ALL_PROXY=p2 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 10 | all_proxy=p1 ALL_PROXY=p2 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 11 | http_proxy='' all_proxy=p2 | p2 · p2:GET http://127.0.0.1:O/lib.bund | p2 · p2:GET http://127.0.0.1:O/lib.bund |
| 12 | http_proxy no scheme | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 13 | http_proxy garbage, all_proxy=p2 | fail · - | fail · - |
| 14 | unresolvable host via http_proxy | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:GET http://x.bund2.invalid:O/lib.bund |
| 15 | no_proxy=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 16 | NO_PROXY=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 17 | no_proxy=other NO_PROXY=127.0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 18 | no_proxy=127.0.0.1 NO_PROXY=other | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 19 | no_proxy='' NO_PROXY=127.0.0.1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 20 | no_proxy=* | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 21 | no_proxy=a,* | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 22 | no_proxy=a, 127.0.0.1 (space) | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 23 | no_proxy=a 127.0.0.1 (space only) | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 24 | no_proxy=127.0.0.0/8 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 25 | no_proxy=127.0.0.1:OPORT | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 26 | no_proxy=127.0.0 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 27 | no_proxy=0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 28 | host x.bund2.invalid no_proxy=x.bund2.invalid | fail · - | fail · - |
| 29 | … no_proxy=X.BUND2.INVALID | fail · - | fail · - |
| 30 | … no_proxy=bund2.invalid | fail · - | fail · - |
| 31 | … no_proxy=.bund2.invalid | fail · - | fail · - |
| 32 | … no_proxy=*.bund2.invalid | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:GET http://x.bund2.invalid:O/lib.bund |
| 33 | … no_proxy=d2.invalid | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:GET http://x.bund2.invalid:O/lib.bund |
| 34 | … no_proxy=invalid | fail · - | fail · - |
| 35 | … no_proxy=x.bund2.invalid. | fail · - | fail · - |
| 36 | … no_proxy=y.x.bund2.invalid | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:GET http://x.bund2.invalid:O/lib.bund |
| 37 | localhost no_proxy=LOCALHOST | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 38 | all_proxy='' ALL_PROXY=p2 | p2 · p2:GET http://127.0.0.1:O/lib.bund | p2 · p2:GET http://127.0.0.1:O/lib.bund |
| 39 | http_proxy=socks5://p1 | fail · p1:SOCKS5 | fail · - |
| 40 | http_proxy=https://p1 | fail · p1:TLS | fail · - |
| 41 | http_proxy=ftp://p1 | fail · - | fail · - |
| 42 | http_proxy=user:pw@p1 | p1 · p1:GET http://127.0.0.1:O/lib.bund [proxy-auth u:p] | p1 · p1:GET http://127.0.0.1:O/lib.bund [proxy-auth u:p] |
| 43 | no_proxy=' * ' | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 44 | no_proxy=127.0.0.0/0 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 45 | no_proxy=127.0.0.1/0 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 46 | no_proxy=127.0.0.1/32 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 47 | no_proxy=127.0.0.1/33 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 48 | no_proxy=127.9.9.9/8 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 49 | no_proxy=126.0.0.0/7 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 50 | no_proxy=128.0.0.0/7 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 51 | no_proxy=127.0.0.1/x | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 52 | no_proxy=.127.0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 53 | no_proxy=127.0.0.1. | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 54 | no_proxy=127.000.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 55 | host [::1] no proxy vars | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 56 | host [::1] http_proxy | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 57 | host [::1] no_proxy=::1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 58 | host [::1] no_proxy=[::1] | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 59 | host [::1] no_proxy=::/64 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 60 | host [::1] no_proxy=0:0:0:0:0:0:0:1 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 61 | host [::1] no_proxy=::1/128 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 62 | host [::1] no_proxy=::1/0 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 63 | host [::1] no_proxy=::1/64 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 64 | host [::1] no_proxy=::1/127 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 65 | host [::1] no_proxy=::0/127 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 66 | host [::1] no_proxy=::/8 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 67 | host [::1] no_proxy=0::1 | p1 · p1:GET http://[::1]:O/lib.bund | p1 · p1:GET http://[::1]:O/lib.bund |
| 68 | host [0:0:0:0:0:0:0:1] no_proxy=::1 | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 69 | host x.bund2.invalid. no_proxy=bund2.invalid | fail · - | fail · - |
| 70 | … no_proxy=..bund2.invalid | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:GET http://x.bund2.invalid:O/lib.bund |
| 71 | … no_proxy=bund2.invalid.. | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:GET http://x.bund2.invalid:O/lib.bund |
| 72 | … no_proxy=. (dot alone) | p1 · p1:GET http://x.bund2.invalid:O/lib.bund | p1 · p1:GET http://x.bund2.invalid:O/lib.bund |
| 73 | user@host no_proxy=127.0.0.1 | origin · origin:GET /lib.bund [auth u:p] | origin · origin:GET /lib.bund [auth u:p] |
| 74 | proxy 127.0.0.1 | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:GET http://127.0.0.1:O/lib.bund |
| 75 | proxy http://127.0.0.1 | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:GET http://127.0.0.1:O/lib.bund |
| 76 | proxy localhost | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:GET http://127.0.0.1:O/lib.bund |
| 77 | proxy http://localhost | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:GET http://127.0.0.1:O/lib.bund |
| 78 | proxy http://127.0.0.1/ | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:GET http://127.0.0.1:O/lib.bund |
| 79 | proxy u:p@127.0.0.1 | p1080 · p1080:GET http://127.0.0.1:O/lib.bund [proxy-auth u:p] | p1080 · p1080:GET http://127.0.0.1:O/lib.bund [proxy-auth u:p] |
| 80 | proxy 127.0.0.1: | fail · - | fail · - |
| 81 | proxy http://127.0.0.1:/ | p1080 · p1080:GET http://127.0.0.1:O/lib.bund | p1080 · p1080:GET http://127.0.0.1:O/lib.bund |
| 82 | proxy HTTP://127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 83 | proxy http://127.0.0.1:P1/path | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 84 | proxy ' http://127.0.0.1:P1' | fail · - | fail · - |
| 85 | proxy 'http://127.0.0.1:P1 ' | fail · - | fail · - |
| 86 | proxy http://127.0.0.1:P1?q | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 87 | proxy http:127.0.0.1:P1 | fail · - | fail · - |
| 88 | proxy //127.0.0.1:P1 | fail · - | fail · - |
| 89 | proxy http://127.1:P1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
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
| 106 | host 127.0.0.09, no_proxy=127.0.0.1 | p1 · p1:GET http://127.0.0.09:O/lib.bund | p1 · p1:GET http://127.0.0.09:O/lib.bund |
| 107 | host 127.0.0.256, no_proxy=127.0.0.1 | p1 · p1:GET http://127.0.0.256:O/lib.bund | p1 · p1:GET http://127.0.0.256:O/lib.bund |
| 108 | host 127.0.65536, no_proxy=127.0.0.1 | p1 · p1:GET http://127.0.65536:O/lib.bund | p1 · p1:GET http://127.0.65536:O/lib.bund |
| 109 | host 127.0.0.1.1, no_proxy=127.0.0.1 | p1 · p1:GET http://127.0.0.1.1:O/lib.bund | p1 · p1:GET http://127.0.0.1.1:O/lib.bund |
| 110 | host 1.2.3.4.5, no_proxy=127.0.0.1 | p1 · p1:GET http://1.2.3.4.5:O/lib.bund | p1 · p1:GET http://1.2.3.4.5:O/lib.bund |
| 111 | host 127..1, no_proxy=127.0.0.1 | p1 · p1:GET http://127..1:O/lib.bund | p1 · p1:GET http://127..1:O/lib.bund |
| 112 | host 0, no_proxy=127.0.0.1 | p1 · p1:GET http://0.0.0.0:O/lib.bund | p1 · p1:GET http://0.0.0.0:O/lib.bund |
| 113 | host localhost, no_proxy=127.0.0.1 | p1 · p1:GET http://localhost:O/lib.bund | p1 · p1:GET http://localhost:O/lib.bund |
| 114 | host LocalHost, no_proxy=127.0.0.1 | p1 · p1:GET http://LocalHost:O/lib.bund | p1 · p1:GET http://LocalHost:O/lib.bund |
| 115 | host 127.1, no proxy at all | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 116 | host 127.0.0.1, no_proxy=127.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 117 | host 127.0.0.1, no_proxy=0x7f.0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 118 | host 127.0.0.1, no_proxy=2130706433 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 119 | host 127.0.0.1, no_proxy=0177.0.0.1 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 120 | host 127.0.0.1, no_proxy=127.0.0.010/32 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 121 | url HTTP:// | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 122 | url HTTP:// via http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 123 | url Http:// | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 124 | url Http:// via http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 125 | url hTTp:// | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 126 | url hTTp:// via http_proxy | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 127 | url file:///abs/lib%20file.bund | file · - | file · - |
| 128 | url FILE:///abs/lib%20file.bund | file · - | file · - |
| 129 | url File:///abs/lib%20file.bund | file · - | file · - |
| 130 | url file://localhost/abs/lib%20file.bund | file · - | file · - |
| 131 | url file://LOCALHOST/abs/lib%20file.bund | file · - | file · - |
| 132 | url FILE://LocalHost/abs/lib%20file.bund | file · - | file · - |
| 133 | url file://127.0.0.1/abs/lib%20file.bund | file · - | file · - |
| 134 | url file://abs/lib.bund | fail · - | fail · - |
| 135 | host 127.0.0.10, no_proxy=127.0.0.010 | p1 · p1:GET http://127.0.0.10:O/lib.bund | p1 · p1:GET http://127.0.0.10:O/lib.bund |
| 136 | host 127.0.0.8, no_proxy=127.0.0.010 | p1 · p1:GET http://127.0.0.8:O/lib.bund | p1 · p1:GET http://127.0.0.8:O/lib.bund |
| 137 | host 127.0.0.10, no_proxy=127.0.0.012 | p1 · p1:GET http://127.0.0.10:O/lib.bund | p1 · p1:GET http://127.0.0.10:O/lib.bund |
| 138 | host 127.0.0.1, no_proxy=127.0.0.01 | p1 · p1:GET http://127.0.0.1:O/lib.bund | p1 · p1:GET http://127.0.0.1:O/lib.bund |
| 139 | host 127.0.0.1, no_proxy=127.0.0.1/8x | origin · origin:GET /lib.bund | origin · origin:GET /lib.bund |
| 140 | host 127.0.0.010, no_proxy=127.0.0.8 | fail · - | fail · - |
| 141 | host 127.0.0.010, no_proxy=127.0.0.10 | p1 · p1:GET http://127.0.0.8:O/lib.bund | p1 · p1:GET http://127.0.0.8:O/lib.bund |
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
| 208 | path /a<b | origin · origin:GET /a<b | origin · origin:GET /a<b |
| 209 | path /a>b | origin · origin:GET /a>b | origin · origin:GET /a>b |
| 210 | path /a`b | origin · origin:GET /a`b | origin · origin:GET /a`b |
| 211 | path /a?< | origin · origin:GET /a?< | origin · origin:GET /a?< |
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
| 243 | proxy 127.0.0.1:P1/ | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 244 | proxy 127.0.0.1:P1/x | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 245 | proxy 127.0.0.1:P1?q | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 246 | proxy localhost:P1/x | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 247 | proxy 127.0.0.1/ | p1080 · p1080:GET http://127.0.0.1:O/x | p1080 · p1080:GET http://127.0.0.1:O/x |
| 248 | proxy 127.0.0.1/x | p1080 · p1080:GET http://127.0.0.1:O/x | p1080 · p1080:GET http://127.0.0.1:O/x |
| 249 | proxy http://127.0.0.1:P1/x?q#f | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 250 | proxy http://127.0.0.1:65536 | fail · - | fail · - |
| 251 | proxy http://127.0.0.1:99999 | fail · - | fail · - |
| 252 | proxy http://127.0.0.1:P1x | fail · - | fail · - |
| 253 | proxy 127.0.0.1:99999 | fail · - | fail · - |
| 254 | proxy http://127.0.0.1:0P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 255 | proxy http://a@b@127.0.0.1:P1 | fail · - | fail · - |
| 256 | proxy http://u:p@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p] |
| 257 | proxy http://127.0.0.1:P1:1 | fail · - | fail · - |
| 258 | proxy http://LOCALHOST:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 259 | proxy http://0x7f.1:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 260 | proxy http://[::ffff:127.0.0.1]:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
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
| 290 | host a&#124;b.invalid, via http_proxy | p1 · p1:GET http://a|b.invalid:O/x | p1 · p1:GET http://a|b.invalid:O/x |
| 291 | host a~b.invalid, via http_proxy | p1 · p1:GET http://a~b.invalid:O/x | p1 · p1:GET http://a~b.invalid:O/x |
| 292 | host a_b.invalid, via http_proxy | p1 · p1:GET http://a_b.invalid:O/x | p1 · p1:GET http://a_b.invalid:O/x |
| 293 | host a-b.invalid, via http_proxy | p1 · p1:GET http://a-b.invalid:O/x | p1 · p1:GET http://a-b.invalid:O/x |
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
| 311 | host [0:0:0:0:0:0:0:1], via http_proxy | p1 · p1:GET http://[::1]:O/x | p1 · p1:GET http://[::1]:O/x |
| 312 | host [0:0:0:0:0:0:0:2], via http_proxy | p1 · p1:GET http://[::2]:O/x | p1 · p1:GET http://[::2]:O/x |
| 313 | host [::2], via http_proxy | p1 · p1:GET http://[::2]:O/x | p1 · p1:GET http://[::2]:O/x |
| 314 | host [::ffff:7f00:1], via http_proxy | p1 · p1:GET http://[::ffff:7f00:1]:O/x | p1 · p1:GET http://[::ffff:7f00:1]:O/x |
| 315 | host [::FFFF:127.0.0.1], via http_proxy | p1 · p1:GET http://[::FFFF:127.0.0.1]:O/x | p1 · p1:GET http://[::FFFF:127.0.0.1]:O/x |
| 316 | host [0:0:0:0:0:0:0:A], via http_proxy | p1 · p1:GET http://[::a]:O/x | p1 · p1:GET http://[::a]:O/x |
| 317 | host [::A], via http_proxy | p1 · p1:GET http://[::A]:O/x | p1 · p1:GET http://[::A]:O/x |
| 318 | host [1:0:0:2:0:0:0:3], via http_proxy | p1 · p1:GET http://[1:0:0:2::3]:O/x | p1 · p1:GET http://[1:0:0:2::3]:O/x |
| 319 | host [1::2:0:0:0:3], via http_proxy | p1 · p1:GET http://[1:0:0:2::3]:O/x | p1 · p1:GET http://[1:0:0:2::3]:O/x |
| 320 | host [0:0:1::], via http_proxy | p1 · p1:GET http://[0:0:1::]:O/x | p1 · p1:GET http://[0:0:1::]:O/x |
| 321 | host [::1.2.3.4], via http_proxy | p1 · p1:GET http://[::1.2.3.4]:O/x | p1 · p1:GET http://[::1.2.3.4]:O/x |
| 322 | host [0:0:0:0:0:0:102:304], via http_proxy | p1 · p1:GET http://[::1.2.3.4]:O/x | p1 · p1:GET http://[::1.2.3.4]:O/x |
| 323 | host [::ffff:7f00:1], no_proxy=::ffff:7f00:1 | origin · origin:GET /x | origin · origin:GET /x |
| 324 | host [::ffff:7f00:1], no_proxy=::ffff:127.0.0.1 | p1 · p1:GET http://[::ffff:7f00:1]:O/x | p1 · p1:GET http://[::ffff:7f00:1]:O/x |
| 325 | host [::FFFF:127.0.0.1], no_proxy=::ffff:127.0.0.1 | origin · origin:GET /x | origin · origin:GET /x |
| 326 | host [::FFFF:127.0.0.1], no_proxy=0.1 | origin · origin:GET /x | origin · origin:GET /x |
| 327 | host [::FFFF:127.0.0.1], no_proxy=127.0.0.1 | p1 · p1:GET http://[::FFFF:127.0.0.1]:O/x | p1 · p1:GET http://[::FFFF:127.0.0.1]:O/x |
| 328 | host [0:0:0:0:0:0:0:1], no_proxy=0:0:0:0:0:0:0:1 | p1 · p1:GET http://[::1]:O/x | p1 · p1:GET http://[::1]:O/x |
| 329 | host [::0001], no_proxy=::1 | origin · origin:GET /x | origin · origin:GET /x |
| 330 | host [::0001], no_proxy=::0001 | p1 · p1:GET http://[::1]:O/x | p1 · p1:GET http://[::1]:O/x |
| 331 | host [0:0:0:0:0:0:0:2], no_proxy=::2 | fail · - | fail · - |
| 332 | host [0:0:0:0:0:0:0:2], no_proxy=::0.0.0.2 | p1 · p1:GET http://[::2]:O/x | p1 · p1:GET http://[::2]:O/x |
| 333 | host [::2], no_proxy=::0.0.0.2 | p1 · p1:GET http://[::2]:O/x | p1 · p1:GET http://[::2]:O/x |
| 334 | no_proxy=0127.0.0.1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 335 | no_proxy=00127.0.0.1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 336 | no_proxy=127.0.0.0001 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 337 | no_proxy=127.0.0.0000000000000000000000000000001 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 338 | no_proxy=127.0.0.9/+8 | origin · origin:GET /x | origin · origin:GET /x |
| 339 | no_proxy=127.0.0.9/4294967304 | origin · origin:GET /x | origin · origin:GET /x |
| 340 | no_proxy=127.0.0.1/+33 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 341 | no_proxy=127.0.0.9/-8 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 342 | no_proxy=127.0.0.1/-0 | origin · origin:GET /x | origin · origin:GET /x |
| 343 | no_proxy=127.0.0.9/-0 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 344 | no_proxy=127.0.0.9/08 | origin · origin:GET /x | origin · origin:GET /x |
| 345 | no_proxy=127.0.0.1/4294967296 | origin · origin:GET /x | origin · origin:GET /x |
| 346 | no_proxy=127.0.0.9/4294967296 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 347 | no_proxy=127.0.0.9/4294967328 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 348 | no_proxy=127.0.0.1/99999999999999999999 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 349 | no_proxy=127.0.0.9/99999999999999999999 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 350 | no_proxy=127.0.0.1/-1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 351 | no_proxy=127.0.0.1/+ | origin · origin:GET /x | origin · origin:GET /x |
| 352 | no_proxy=127.0.0.1/ | origin · origin:GET /x | origin · origin:GET /x |
| 353 | no_proxy=127.0.0.9/8/1 | origin · origin:GET /x | origin · origin:GET /x |
| 354 | no_proxy=127.0.0.1/+0x | origin · origin:GET /x | origin · origin:GET /x |
| 355 | no_proxy=127.0.0.9/++8 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 356 | no_proxy=127.0.0.9/+-8 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 357 | no_proxy=127.0.0.9/-4294967288 | origin · origin:GET /x | origin · origin:GET /x |
| 358 | no_proxy=127.0.0.1/-99999999999999999999 | origin · origin:GET /x | origin · origin:GET /x |
| 359 | no_proxy=127.0.0.9/<newline>8 | origin · origin:GET /x | origin · origin:GET /x |
| 360 | no_proxy=0300.0.0.1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
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
| 389 | url user part `u:p@`, via http_proxy | p1 · p1:GET http://127.0.0.1:O/x [auth u:p] | p1 · p1:GET http://127.0.0.1:O/x [auth u:p] |
| 390 | url user part `a%00b@`, via http_proxy | fail · - | fail · - |
| 391 | url user part `a%40b@`, via http_proxy | p1 · p1:GET http://127.0.0.1:O/x [auth a@b:] | p1 · p1:GET http://127.0.0.1:O/x [auth a@b:] |
| 392 | proxy http://u:p@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p] |
| 393 | proxy http://a%00b@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 394 | proxy http://u:%00@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 395 | proxy http://a%01b@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 396 | proxy http://a%09b@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 397 | proxy http://a%0ab@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 398 | proxy http://a%0db@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 399 | proxy http://a%1fb@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 400 | proxy http://u:%1f@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 401 | proxy http://a%20b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a b:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a b:] |
| 402 | proxy http://a%7fb@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\x7fb:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\x7fb:] |
| 403 | proxy http://a%80b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\x80b:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\x80b:] |
| 404 | proxy http://a%ffb@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\xffb:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a\xffb:] |
| 405 | proxy http://a%40b:c%3Ad@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a@b:c:d] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a@b:c:d] |
| 406 | proxy http://u%20s@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u s:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u s:] |
| 407 | proxy http://%zz@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth %zz:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth %zz:] |
| 408 | proxy http://a%@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a%:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a%:] |
| 409 | proxy http://a%4@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a%4:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a%4:] |
| 410 | proxy http://a<b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a<b:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a<b:] |
| 411 | proxy http://a&#124;b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a|b:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a|b:] |
| 412 | proxy http://é@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth \xc3\xa9:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth \xc3\xa9:] |
| 413 | proxy http://@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :] |
| 414 | proxy http://:@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :] |
| 415 | proxy http://u:@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:] |
| 416 | proxy http://:p@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :p] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth :p] |
| 417 | proxy http://a:b:c@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a:b:c] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a:b:c] |
| 418 | proxy http://a;b@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a;b:] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth a;b:] |
| 419 | proxy http://u:p%2@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p%2] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p%2] |
| 420 | proxy a%00b@127.0.0.1:P1, no scheme | origin · origin:GET /x | fail · - |
| 421 | all_proxy http://a%00b@127.0.0.1:P1 | origin · origin:GET /x | fail · - |
| 422 | http_proxy http://a%00b@127.0.0.1:P1, all_proxy=p2 | origin · origin:GET /x | fail · - |
| 423 | proxy http://127.0.0.1:P1#f | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
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
| 438 | path /h129 | origin · origin:GET /h129 | origin · origin:GET /h129 |
| 439 | path /h130 | origin · origin:GET /h130 | origin · origin:GET /h130 |
| 440 | path /h200 | origin · origin:GET /h200 | origin · origin:GET /h200 |
| 441 | path /v2 | origin · origin:GET /v2 | origin · origin:GET /v2 |
| 442 | path /a?<> | origin · origin:GET /a?<> | origin · origin:GET /a?<> |
| 443 | a proxy that refuses CONNECT and answers GET | p3 · p3:GET http://127.0.0.1:O/x | p3 · p3:GET http://127.0.0.1:O/x |
| 444 | response 302-length | body · origin:GET /r/302-length | body · origin:GET /r/302-length |
| 445 | response 302-no-length | body · origin:GET /r/302-no-length | body · origin:GET /r/302-no-length |
| 446 | response 301-no-length | body · origin:GET /r/301-no-length | body · origin:GET /r/301-no-length |
| 447 | response 303-no-length | body · origin:GET /r/303-no-length | body · origin:GET /r/303-no-length |
| 448 | response 307-no-length | body · origin:GET /r/307-no-length | body · origin:GET /r/307-no-length |
| 449 | response 308-no-length | body · origin:GET /r/308-no-length | body · origin:GET /r/308-no-length |
| 450 | response 302-no-location | body · origin:GET /r/302-no-location | body · origin:GET /r/302-no-location |
| 451 | response 300-no-length | body · origin:GET /r/300-no-length | body · origin:GET /r/300-no-length |
| 452 | response 304-no-length | empty · origin:GET /r/304-no-length | empty · origin:GET /r/304-no-length |
| 453 | response 304-length | empty · origin:GET /r/304-length | empty · origin:GET /r/304-length |
| 454 | response 204-no-length | empty · origin:GET /r/204-no-length | empty · origin:GET /r/204-no-length |
| 455 | response 204-length | empty · origin:GET /r/204-length | empty · origin:GET /r/204-length |
| 456 | response 101 | body · origin:GET /r/101 | body · origin:GET /r/101 |
| 457 | response 100-then-200 | body · origin:GET /r/100-then-200 | body · origin:GET /r/100-then-200 |
| 458 | response 103-then-200 | body · origin:GET /r/103-then-200 | body · origin:GET /r/103-then-200 |
| 459 | response 100-alone | fail · origin:GET /r/100-alone | fail · origin:GET /r/100-alone |
| 460 | response 404 | body · origin:GET /r/404 | body · origin:GET /r/404 |
| 461 | response 500-no-length | body · origin:GET /r/500-no-length | body · origin:GET /r/500-no-length |
| 462 | response 999 | body · origin:GET /r/999 | body · origin:GET /r/999 |
| 463 | response 99 | fail · origin:GET /r/99 | fail · origin:GET /r/99 |
| 464 | response 1000 | fail · origin:GET /r/1000 | fail · origin:GET /r/1000 |
| 465 | response no-reason | body · origin:GET /r/no-reason | body · origin:GET /r/no-reason |
| 466 | response http-1.0 | body · origin:GET /r/http-1.0 | body · origin:GET /r/http-1.0 |
| 467 | response http-1.0-no-length | body · origin:GET /r/http-1.0-no-length | body · origin:GET /r/http-1.0-no-length |
| 468 | response http-1.2 | fail · origin:GET /r/http-1.2 | fail · origin:GET /r/http-1.2 |
| 469 | response http-3 | fail · origin:GET /r/http-3 | fail · origin:GET /r/http-3 |
| 470 | response lower-case-http | body · origin:GET /r/lower-case-http | body · origin:GET /r/lower-case-http |
| 471 | response icy | fail · origin:GET /r/icy | fail · origin:GET /r/icy |
| 472 | response no-status-line | fail · origin:GET /r/no-status-line | fail · origin:GET /r/no-status-line |
| 473 | response status-line-alone | empty · origin:GET /r/status-line-alone | empty · origin:GET /r/status-line-alone |
| 474 | response status-line-and-blank | empty · origin:GET /r/status-line-and-blank | empty · origin:GET /r/status-line-and-blank |
| 475 | response nothing | fail · origin:GET /r/nothing | fail · origin:GET /r/nothing |
| 476 | response blank-line-first | fail · origin:GET /r/blank-line-first | fail · origin:GET /r/blank-line-first |
| 477 | response two-spaces-in-status | fail · origin:GET /r/two-spaces-in-status | fail · origin:GET /r/two-spaces-in-status |
| 478 | response tab-in-status | body · origin:GET /r/tab-in-status | body · origin:GET /r/tab-in-status |
| 479 | response no-length | body · origin:GET /r/no-length | body · origin:GET /r/no-length |
| 480 | response no-length-no-close | body · origin:GET /r/no-length-no-close | body · origin:GET /r/no-length-no-close |
| 481 | response length-0 | empty · origin:GET /r/length-0 | empty · origin:GET /r/length-0 |
| 482 | response length-plus | body · origin:GET /r/length-plus | body · origin:GET /r/length-plus |
| 483 | response length-minus | fail · origin:GET /r/length-minus | fail · origin:GET /r/length-minus |
| 484 | response length-then-letters | body · origin:GET /r/length-then-letters | body · origin:GET /r/length-then-letters |
| 485 | response length-letters | fail · origin:GET /r/length-letters | fail · origin:GET /r/length-letters |
| 486 | response length-hex | empty · origin:GET /r/length-hex | empty · origin:GET /r/length-hex |
| 487 | response length-past-64-bits | body · origin:GET /r/length-past-64-bits | body · origin:GET /r/length-past-64-bits |
| 488 | response length-empty | fail · origin:GET /r/length-empty | fail · origin:GET /r/length-empty |
| 489 | response length-blanks | body · origin:GET /r/length-blanks | body · origin:GET /r/length-blanks |
| 490 | response length-leading-zeros | body · origin:GET /r/length-leading-zeros | body · origin:GET /r/length-leading-zeros |
| 491 | response length-twice-same | body · origin:GET /r/length-twice-same | body · origin:GET /r/length-twice-same |
| 492 | response length-twice-short-first | body · origin:GET /r/length-twice-short-first | body · origin:GET /r/length-twice-short-first |
| 493 | response length-twice-long-first | `"ori` · origin:GET /r/length-twice-long-first | `"ori` · origin:GET /r/length-twice-long-first |
| 494 | response length-list | body · origin:GET /r/length-list | body · origin:GET /r/length-list |
| 495 | response length-short | `"ori` · origin:GET /r/length-short | `"ori` · origin:GET /r/length-short |
| 496 | response length-long | fail · origin:GET /r/length-long | fail · origin:GET /r/length-long |
| 497 | response chunked | body · origin:GET /r/chunked | body · origin:GET /r/chunked |
| 498 | response chunked-and-length | body · origin:GET /r/chunked-and-length | body · origin:GET /r/chunked-and-length |
| 499 | response chunked-twice | body · origin:GET /r/chunked-twice | fail · origin:GET /r/chunked-twice |
| 500 | response chunked-chunked | body · origin:GET /r/chunked-chunked | fail · origin:GET /r/chunked-chunked |
| 501 | response gzip-chunked | `11\n"origin" println\n\n0\n\n` · origin:GET /r/gzip-chunked | `11\n"origin" println\n\n0\n\n` · origin:GET /r/gzip-chunked |
| 502 | response identity-coding | body · origin:GET /r/identity-coding | body · origin:GET /r/identity-coding |
| 503 | response unknown-coding | body · origin:GET /r/unknown-coding | body · origin:GET /r/unknown-coding |
| 504 | response chunked-upper-case | body · origin:GET /r/chunked-upper-case | body · origin:GET /r/chunked-upper-case |
| 505 | response chunked-http-1.0 | body · origin:GET /r/chunked-http-1.0 | body · origin:GET /r/chunked-http-1.0 |
| 506 | response chunk-size-plus | fail · origin:GET /r/chunk-size-plus | fail · origin:GET /r/chunk-size-plus |
| 507 | response chunk-size-blank-before | fail · origin:GET /r/chunk-size-blank-before | fail · origin:GET /r/chunk-size-blank-before |
| 508 | response chunk-size-blank-after | body · origin:GET /r/chunk-size-blank-after | body · origin:GET /r/chunk-size-blank-after |
| 509 | response chunk-size-upper-case | body · origin:GET /r/chunk-size-upper-case | body · origin:GET /r/chunk-size-upper-case |
| 510 | response chunk-size-0x | fail · origin:GET /r/chunk-size-0x | fail · origin:GET /r/chunk-size-0x |
| 511 | response chunk-size-leading-zeros | body · origin:GET /r/chunk-size-leading-zeros | body · origin:GET /r/chunk-size-leading-zeros |
| 512 | response chunk-size-17-digits | fail · origin:GET /r/chunk-size-17-digits | fail · origin:GET /r/chunk-size-17-digits |
| 513 | response chunk-extension | body · origin:GET /r/chunk-extension | body · origin:GET /r/chunk-extension |
| 514 | response chunk-size-not-hex | fail · origin:GET /r/chunk-size-not-hex | fail · origin:GET /r/chunk-size-not-hex |
| 515 | response chunk-line-feeds | body · origin:GET /r/chunk-line-feeds | body · origin:GET /r/chunk-line-feeds |
| 516 | response chunk-no-last | fail · origin:GET /r/chunk-no-last | fail · origin:GET /r/chunk-no-last |
| 517 | response chunk-short | fail · origin:GET /r/chunk-short | fail · origin:GET /r/chunk-short |
| 518 | response chunk-trailer | body · origin:GET /r/chunk-trailer | body · origin:GET /r/chunk-trailer |
| 519 | response chunk-no-crlf-after-data | fail · origin:GET /r/chunk-no-crlf-after-data | fail · origin:GET /r/chunk-no-crlf-after-data |
| 520 | response chunk-two | body · origin:GET /r/chunk-two | body · origin:GET /r/chunk-two |
| 521 | response line-feeds | body · origin:GET /r/line-feeds | body · origin:GET /r/line-feeds |
| 522 | response folded-header | body · origin:GET /r/folded-header | body · origin:GET /r/folded-header |
| 523 | response folded-length | fail · origin:GET /r/folded-length | fail · origin:GET /r/folded-length |
| 524 | response blank-in-name | body · origin:GET /r/blank-in-name | body · origin:GET /r/blank-in-name |
| 525 | response blank-before-colon | body · origin:GET /r/blank-before-colon | body · origin:GET /r/blank-before-colon |
| 526 | response empty-name | body · origin:GET /r/empty-name | body · origin:GET /r/empty-name |
| 527 | response no-colon | fail · origin:GET /r/no-colon | fail · origin:GET /r/no-colon |
| 528 | response carriage-return-in-value | body · origin:GET /r/carriage-return-in-value | body · origin:GET /r/carriage-return-in-value |
| 529 | response nul-in-value | fail · origin:GET /r/nul-in-value | fail · origin:GET /r/nul-in-value |
| 530 | response high-byte-in-value | body · origin:GET /r/high-byte-in-value | body · origin:GET /r/high-byte-in-value |
| 531 | response high-byte-in-name | body · origin:GET /r/high-byte-in-name | body · origin:GET /r/high-byte-in-name |
| 532 | response header-100000-bytes | body · origin:GET /r/header-100000-bytes | body · origin:GET /r/header-100000-bytes |
| 533 | response header-400000-bytes | fail · origin:GET /r/header-400000-bytes | fail · origin:GET /r/header-400000-bytes |
| 534 | response gzip-content | body · origin:GET /r/gzip-content | body · origin:GET /r/gzip-content |
| 535 | response keep-alive | body · origin:GET /r/keep-alive | body · origin:GET /r/keep-alive |
| 536 | response bytes-after-body | body · origin:GET /r/bytes-after-body | body · origin:GET /r/bytes-after-body |
| 537 | response not-utf-8 | `"origin" println\n�\n` · origin:GET /r/not-utf-8 | `"origin" println\n�\n` · origin:GET /r/not-utf-8 |
| 538 | response blank-before-status | fail · origin:GET /r/blank-before-status | fail · origin:GET /r/blank-before-status |
| 539 | response http-2 | body · origin:GET /r/http-2 | body · origin:GET /r/http-2 |
| 540 | response http-2-no-length | body · origin:GET /r/http-2-no-length | body · origin:GET /r/http-2-no-length |
| 541 | response http-2-chunked | body · origin:GET /r/http-2-chunked | body · origin:GET /r/http-2-chunked |
| 542 | response http-2-no-reason | body · origin:GET /r/http-2-no-reason | body · origin:GET /r/http-2-no-reason |
| 543 | response http-2.0 | body · origin:GET /r/http-2.0 | body · origin:GET /r/http-2.0 |
| 544 | response http-2-two-digits | body · origin:GET /r/http-2-two-digits | body · origin:GET /r/http-2-two-digits |
| 545 | response http-2-four-digits | body · origin:GET /r/http-2-four-digits | body · origin:GET /r/http-2-four-digits |
| 546 | response http-2-code-99 | fail · origin:GET /r/http-2-code-99 | fail · origin:GET /r/http-2-code-99 |
| 547 | response http-3-two-digits | body · origin:GET /r/http-3-two-digits | body · origin:GET /r/http-3-two-digits |
| 548 | response http-3.0 | body · origin:GET /r/http-3.0 | body · origin:GET /r/http-3.0 |
| 549 | response http-4 | fail · origin:GET /r/http-4 | fail · origin:GET /r/http-4 |
| 550 | response http-1 | fail · origin:GET /r/http-1 | fail · origin:GET /r/http-1 |
| 551 | response http-0.9 | fail · origin:GET /r/http-0.9 | fail · origin:GET /r/http-0.9 |
| 552 | response http-1.1-no-blank | fail · origin:GET /r/http-1.1-no-blank | fail · origin:GET /r/http-1.1-no-blank |
| 553 | response http-no-version | fail · origin:GET /r/http-no-version | fail · origin:GET /r/http-no-version |
| 554 | response lower-case-http-304 | body · origin:GET /r/lower-case-http-304 | body · origin:GET /r/lower-case-http-304 |
| 555 | response lower-case-http-1.2 | body · origin:GET /r/lower-case-http-1.2 | body · origin:GET /r/lower-case-http-1.2 |
| 556 | response lower-case-http-chunked | body · origin:GET /r/lower-case-http-chunked | body · origin:GET /r/lower-case-http-chunked |
| 557 | response mixed-case-http | body · origin:GET /r/mixed-case-http | body · origin:GET /r/mixed-case-http |
| 558 | response code-letters | fail · origin:GET /r/code-letters | fail · origin:GET /r/code-letters |
| 559 | response code-then-tab | body · origin:GET /r/code-then-tab | body · origin:GET /r/code-then-tab |
| 560 | response code-then-letter | fail · origin:GET /r/code-then-letter | fail · origin:GET /r/code-then-letter |
| 561 | response code-099 | fail · origin:GET /r/code-099 | fail · origin:GET /r/code-099 |
| 562 | response code-000 | fail · origin:GET /r/code-000 | fail · origin:GET /r/code-000 |
| 563 | response nul-in-status | fail · origin:GET /r/nul-in-status | fail · origin:GET /r/nul-in-status |
| 564 | response status-no-newline | fail · origin:GET /r/status-no-newline | fail · origin:GET /r/status-no-newline |
| 565 | response status-line-feed-alone | empty · origin:GET /r/status-line-feed-alone | empty · origin:GET /r/status-line-feed-alone |
| 566 | response three-bytes | fail · origin:GET /r/three-bytes | fail · origin:GET /r/three-bytes |
| 567 | response five-bytes | fail · origin:GET /r/five-bytes | fail · origin:GET /r/five-bytes |
| 568 | response ends-after-length | fail · origin:GET /r/ends-after-length | fail · origin:GET /r/ends-after-length |
| 569 | response ends-after-length-0 | empty · origin:GET /r/ends-after-length-0 | empty · origin:GET /r/ends-after-length-0 |
| 570 | response ends-after-chunked | fail · origin:GET /r/ends-after-chunked | empty · origin:GET /r/ends-after-chunked |
| 571 | response ends-in-a-header | empty · origin:GET /r/ends-in-a-header | empty · origin:GET /r/ends-in-a-header |
| 572 | response ends-in-a-header-after-length | fail · origin:GET /r/ends-in-a-header-after-length | fail · origin:GET /r/ends-in-a-header-after-length |
| 573 | response ends-in-a-length | empty · origin:GET /r/ends-in-a-length | empty · origin:GET /r/ends-in-a-length |
| 574 | response 103-alone | fail · origin:GET /r/103-alone | fail · origin:GET /r/103-alone |
| 575 | response 100-then-garbage | fail · origin:GET /r/100-then-garbage | fail · origin:GET /r/100-then-garbage |
| 576 | response 100-twice-then-200 | body · origin:GET /r/100-twice-then-200 | body · origin:GET /r/100-twice-then-200 |
| 577 | response 100-length-then-200 | body · origin:GET /r/100-length-then-200 | body · origin:GET /r/100-length-then-200 |
| 578 | response 100-chunked-then-200 | body · origin:GET /r/100-chunked-then-200 | body · origin:GET /r/100-chunked-then-200 |
| 579 | response 100-then-status-alone | empty · origin:GET /r/100-then-status-alone | empty · origin:GET /r/100-then-status-alone |
| 580 | response 103-then-status-alone | empty · origin:GET /r/103-then-status-alone | empty · origin:GET /r/103-then-status-alone |
| 581 | response 199-then-200 | body · origin:GET /r/199-then-200 | body · origin:GET /r/199-then-200 |
| 582 | response 100-then-lower-case-http | body · origin:GET /r/100-then-lower-case-http | body · origin:GET /r/100-then-lower-case-http |
| 583 | response 101-length | body · origin:GET /r/101-length | body · origin:GET /r/101-length |
| 584 | response 101-chunked | `11\n"origin" println\n\n0\n\n` · origin:GET /r/101-chunked | `11\n"origin" println\n\n0\n\n` · origin:GET /r/101-chunked |
| 585 | response 204-chunked | empty · origin:GET /r/204-chunked | empty · origin:GET /r/204-chunked |
| 586 | response 304-chunked | empty · origin:GET /r/304-chunked | empty · origin:GET /r/304-chunked |
| 587 | response 204-bad-length | empty · origin:GET /r/204-bad-length | empty · origin:GET /r/204-bad-length |
| 588 | response 401-basic | body · origin:GET /r/401-basic | body · origin:GET /r/401-basic |
| 589 | response 401-digest | body · origin:GET /r/401-digest | body · origin:GET /r/401-digest |
| 590 | response 407-basic | body · origin:GET /r/407-basic | body · origin:GET /r/407-basic |
| 591 | response first-header-folded | body · origin:GET /r/first-header-folded | body · origin:GET /r/first-header-folded |
| 592 | response first-header-folded-no-colon | fail · origin:GET /r/first-header-folded-no-colon | fail · origin:GET /r/first-header-folded-no-colon |
| 593 | response folded-with-colon | body · origin:GET /r/folded-with-colon | body · origin:GET /r/folded-with-colon |
| 594 | response folded-length-after | body · origin:GET /r/folded-length-after | body · origin:GET /r/folded-length-after |
| 595 | response header-begins-with-carriage-return | `Content-Length: 17\nConnection: close\n\n"origin" println\n` · origin:GET /r/header-begins-with-carriage-return | `Content-Length: 17\nConnection: close\n\n"origin" println\n` · origin:GET /r/header-begins-with-carriage-return |
| 596 | response carriage-returns-alone | empty · origin:GET /r/carriage-returns-alone | empty · origin:GET /r/carriage-returns-alone |
| 597 | response length-lower-case | body · origin:GET /r/length-lower-case | body · origin:GET /r/length-lower-case |
| 598 | response length-tab | body · origin:GET /r/length-tab | body · origin:GET /r/length-tab |
| 599 | response length-no-blank | body · origin:GET /r/length-no-blank | body · origin:GET /r/length-no-blank |
| 600 | response length-inner-blank | `"` · origin:GET /r/length-inner-blank | `"` · origin:GET /r/length-inner-blank |
| 601 | response length-plus-plus | fail · origin:GET /r/length-plus-plus | fail · origin:GET /r/length-plus-plus |
| 602 | response length-plus-alone | fail · origin:GET /r/length-plus-alone | fail · origin:GET /r/length-plus-alone |
| 603 | response length-minus-0 | fail · origin:GET /r/length-minus-0 | fail · origin:GET /r/length-minus-0 |
| 604 | response length-largest | fail · origin:GET /r/length-largest | fail · origin:GET /r/length-largest |
| 605 | response length-largest-and-1 | body · origin:GET /r/length-largest-and-1 | body · origin:GET /r/length-largest-and-1 |
| 606 | response length-vertical-tab | fail · origin:GET /r/length-vertical-tab | fail · origin:GET /r/length-vertical-tab |
| 607 | response length-then-bad-length | fail · origin:GET /r/length-then-bad-length | fail · origin:GET /r/length-then-bad-length |
| 608 | response length-prefix-name | body · origin:GET /r/length-prefix-name | body · origin:GET /r/length-prefix-name |
| 609 | response length-then-chunked | body · origin:GET /r/length-then-chunked | body · origin:GET /r/length-then-chunked |
| 610 | response chunked-then-gzip | body · origin:GET /r/chunked-then-gzip | body · origin:GET /r/chunked-then-gzip |
| 611 | response x-then-chunked-two-lines | body · origin:GET /r/x-then-chunked-two-lines | body · origin:GET /r/x-then-chunked-two-lines |
| 612 | response chunked-then-x-two-lines | body · origin:GET /r/chunked-then-x-two-lines | body · origin:GET /r/chunked-then-x-two-lines |
| 613 | response identity-chunked | `11\n"origin" println\n\n0\n\n` · origin:GET /r/identity-chunked | `11\n"origin" println\n\n0\n\n` · origin:GET /r/identity-chunked |
| 614 | response chunked-parameter | `11\n"origin" println\n\n0\n\n` · origin:GET /r/chunked-parameter | `11\n"origin" println\n\n0\n\n` · origin:GET /r/chunked-parameter |
| 615 | response comma-chunked | body · origin:GET /r/comma-chunked | body · origin:GET /r/comma-chunked |
| 616 | response chunked-comma | body · origin:GET /r/chunked-comma | body · origin:GET /r/chunked-comma |
| 617 | response chunked-no-blank | body · origin:GET /r/chunked-no-blank | body · origin:GET /r/chunked-no-blank |
| 618 | response chunked-tab | body · origin:GET /r/chunked-tab | body · origin:GET /r/chunked-tab |
| 619 | response chunkedx | `11\n"origin" println\n\n0\n\n` · origin:GET /r/chunkedx | `11\n"origin" println\n\n0\n\n` · origin:GET /r/chunkedx |
| 620 | response coding-empty | body · origin:GET /r/coding-empty | body · origin:GET /r/coding-empty |
| 621 | response chunked-twice-and-twice | `11\n"origin" println\n\n0\n\n` · origin:GET /r/chunked-twice-and-twice | body · origin:GET /r/chunked-twice-and-twice |
| 622 | response chunked-thrice-and-thrice | `1c\n11\n"origin" println\n\n0\n\n\n0\n\n` · origin:GET /r/chunked-thrice-and-thrice | body · origin:GET /r/chunked-thrice-and-thrice |
| 623 | response chunked-once-and-twice | `11\n"origin" println\n\n0\n\n` · origin:GET /r/chunked-once-and-twice | `11\n"origin" println\n\n0\n\n` · origin:GET /r/chunked-once-and-twice |
| 624 | response chunk-size-16-digits | body · origin:GET /r/chunk-size-16-digits | body · origin:GET /r/chunk-size-16-digits |
| 625 | response chunk-size-empty | fail · origin:GET /r/chunk-size-empty | fail · origin:GET /r/chunk-size-empty |
| 626 | response chunk-size-then-letter | body · origin:GET /r/chunk-size-then-letter | body · origin:GET /r/chunk-size-then-letter |
| 627 | response chunk-size-tab-after | body · origin:GET /r/chunk-size-tab-after | body · origin:GET /r/chunk-size-tab-after |
| 628 | response chunk-size-minus | fail · origin:GET /r/chunk-size-minus | fail · origin:GET /r/chunk-size-minus |
| 629 | response chunk-zero-alone | empty · origin:GET /r/chunk-zero-alone | empty · origin:GET /r/chunk-zero-alone |
| 630 | response chunk-two-carriage-returns-after-data | body · origin:GET /r/chunk-two-carriage-returns-after-data | body · origin:GET /r/chunk-two-carriage-returns-after-data |
| 631 | response chunk-last-00 | body · origin:GET /r/chunk-last-00 | body · origin:GET /r/chunk-last-00 |
| 632 | response chunk-last-extension | body · origin:GET /r/chunk-last-extension | body · origin:GET /r/chunk-last-extension |
| 633 | response chunk-last-line-feeds | body · origin:GET /r/chunk-last-line-feeds | body · origin:GET /r/chunk-last-line-feeds |
| 634 | response chunk-last-no-final-line | fail · origin:GET /r/chunk-last-no-final-line | fail · origin:GET /r/chunk-last-no-final-line |
| 635 | response chunk-last-final-carriage-return | body · origin:GET /r/chunk-last-final-carriage-return | fail · origin:GET /r/chunk-last-final-carriage-return |
| 636 | response chunk-last-final-two-carriage-returns | fail · origin:GET /r/chunk-last-final-two-carriage-returns | fail · origin:GET /r/chunk-last-final-two-carriage-returns |
| 637 | response chunk-bytes-after-last | body · origin:GET /r/chunk-bytes-after-last | body · origin:GET /r/chunk-bytes-after-last |
| 638 | response chunk-trailer-no-colon | fail · origin:GET /r/chunk-trailer-no-colon | fail · origin:GET /r/chunk-trailer-no-colon |
| 639 | response chunk-trailer-two | body · origin:GET /r/chunk-trailer-two | body · origin:GET /r/chunk-trailer-two |
| 640 | response chunk-trailer-line-feeds | body · origin:GET /r/chunk-trailer-line-feeds | body · origin:GET /r/chunk-trailer-line-feeds |
| 641 | response chunk-trailer-folded | body · origin:GET /r/chunk-trailer-folded | body · origin:GET /r/chunk-trailer-folded |
| 642 | response chunk-trailer-begins-blank | body · origin:GET /r/chunk-trailer-begins-blank | body · origin:GET /r/chunk-trailer-begins-blank |
| 643 | response chunk-trailer-nul | fail · origin:GET /r/chunk-trailer-nul | fail · origin:GET /r/chunk-trailer-nul |
| 644 | response chunk-trailer-carriage-return-inside | fail · origin:GET /r/chunk-trailer-carriage-return-inside | fail · origin:GET /r/chunk-trailer-carriage-return-inside |
| 645 | response chunk-trailer-unfinished | fail · origin:GET /r/chunk-trailer-unfinished | fail · origin:GET /r/chunk-trailer-unfinished |
| 646 | response chunk-trailer-5000-bytes | fail · origin:GET /r/chunk-trailer-5000-bytes | fail · origin:GET /r/chunk-trailer-5000-bytes |
| 647 | response 101-alone | fail · origin:GET /r/101-alone | fail · origin:GET /r/101-alone |
| 648 | response 100-then-status-no-newline | fail · origin:GET /r/100-then-status-no-newline | fail · origin:GET /r/100-then-status-no-newline |
| 649 | response 204-ends-in-head | empty · origin:GET /r/204-ends-in-head | empty · origin:GET /r/204-ends-in-head |
| 650 | response line-feed-first | fail · origin:GET /r/line-feed-first | fail · origin:GET /r/line-feed-first |
| 651 | response http-2-tab | body · origin:GET /r/http-2-tab | body · origin:GET /r/http-2-tab |
| 652 | response colon-after-carriage-return | fail · origin:GET /r/colon-after-carriage-return | fail · origin:GET /r/colon-after-carriage-return |
| 653 | response chunk-trailer-empty-name | body · origin:GET /r/chunk-trailer-empty-name | body · origin:GET /r/chunk-trailer-empty-name |
| 654 | response chunk-trailer-blanks | body · origin:GET /r/chunk-trailer-blanks | body · origin:GET /r/chunk-trailer-blanks |
| 655 | response chunked-4-and-4 | `27\n1c\n11\n"origin" println\n\n0\n\n\n0\n\n\n0\n\n` · origin:GET /r/chunked-4-and-4 | body · origin:GET /r/chunked-4-and-4 |
| 656 | response chunked-5-and-5 | `32\n27\n1c\n11\n"origin" println\n\n0\n\n\n0\n\n\n0\n\n\n0\n` · origin:GET /r/chunked-5-and-5 | fail · origin:GET /r/chunked-5-and-5 |
| 657 | response chunked-6-and-6 | `3d\n32\n27\n1c\n11\n"origin" println\n\n0\n\n\n0\n\n\n0\n\n\` · origin:GET /r/chunked-6-and-6 | fail · origin:GET /r/chunked-6-and-6 |
| 658 | response chunked-twice-inner-unfinished | `11\n"origin" println\n\n` · origin:GET /r/chunked-twice-inner-unfinished | body · origin:GET /r/chunked-twice-inner-unfinished |
| 659 | response chunked-twice-inner-bytes-after | `11\n"origin" println\n\n0\n\nextra` · origin:GET /r/chunked-twice-inner-bytes-after | body · origin:GET /r/chunked-twice-inner-bytes-after |
| 660 | response chunked-twice-outer-unfinished | fail · origin:GET /r/chunked-twice-outer-unfinished | body · origin:GET /r/chunked-twice-outer-unfinished |
| 661 | response chunked-twice-outer-bad-after-inner | fail · origin:GET /r/chunked-twice-outer-bad-after-inner | fail · origin:GET /r/chunked-twice-outer-bad-after-inner |
| 662 | response 100-12000-times | body · origin:GET /r/100-12000-times | body · origin:GET /r/100-12000-times |
| 663 | response 100-12300-times | fail · origin:GET /r/100-12300-times | fail · origin:GET /r/100-12300-times |
| 664 | response line-102398 | body · origin:GET /r/line-102398 | body · origin:GET /r/line-102398 |
| 665 | response line-102399 | body · origin:GET /r/line-102399 | body · origin:GET /r/line-102399 |
| 666 | response line-102400 | fail · origin:GET /r/line-102400 | fail · origin:GET /r/line-102400 |
| 667 | response line-102401 | fail · origin:GET /r/line-102401 | fail · origin:GET /r/line-102401 |
| 668 | response head-307198 | body · origin:GET /r/head-307198 | body · origin:GET /r/head-307198 |
| 669 | response head-307199 | body · origin:GET /r/head-307199 | body · origin:GET /r/head-307199 |
| 670 | response head-307200 | body · origin:GET /r/head-307200 | body · origin:GET /r/head-307200 |
| 671 | response head-307201 | fail · origin:GET /r/head-307201 | fail · origin:GET /r/head-307201 |
| 672 | response head-307202 | fail · origin:GET /r/head-307202 | fail · origin:GET /r/head-307202 |
| 673 | response trailer-4095 | body · origin:GET /r/trailer-4095 | body · origin:GET /r/trailer-4095 |
| 674 | response trailer-4096 | fail · origin:GET /r/trailer-4096 | fail · origin:GET /r/trailer-4096 |
| 675 | response trailer-4097 | fail · origin:GET /r/trailer-4097 | fail · origin:GET /r/trailer-4097 |
| 676 | response trailer-4098 | fail · origin:GET /r/trailer-4098 | fail · origin:GET /r/trailer-4098 |
| 677 | path /h4999 | origin · origin:GET /h4999 | origin · origin:GET /h4999 |
| 678 | path /h5000 | origin · origin:GET /h5000 | origin · origin:GET /h5000 |
| 679 | path /h5001 | origin · origin:GET /h5001 | origin · origin:GET /h5001 |
| 680 | path /h5002 | origin · origin:GET /h5002 | origin · origin:GET /h5002 |
| 681 | path /h5003 | origin · origin:GET /h5003 | origin · origin:GET /h5003 |
| 682 | response 401-basic, with a user part | body · origin:GET /r/401-basic [auth u:p] | body · origin:GET /r/401-basic [auth u:p] |
| 683 | response 401-digest, with a user part | body · origin:GET /r/401-digest [auth u:p] | body · origin:GET /r/401-digest [auth u:p] |
| 684 | response 404, with a user part | body · origin:GET /r/404 [auth u:p] | body · origin:GET /r/404 [auth u:p] |
| 685 | response 407-basic, from a proxy with credentials | `"p1" println\n` · p1:GET http://127.0.0.1:O/r/407-basic [proxy-auth u:p] | `"p1" println\n` · p1:GET http://127.0.0.1:O/r/407-basic [proxy-auth u:p] |
| 686 | response 302-no-length, by `use` | origin · origin:GET /r/302-no-length | origin · origin:GET /r/302-no-length |
| 687 | response 101, by `use` | origin · origin:GET /r/101 | origin · origin:GET /r/101 |
| 688 | response chunked-twice, by `use` | origin · origin:GET /r/chunked-twice | fail · origin:GET /r/chunked-twice |
| 689 | response gzip-chunked, by `use` | origin · origin:GET /r/gzip-chunked | origin · origin:GET /r/gzip-chunked |
| 690 | response length-twice-short-first, by `use` | origin · origin:GET /r/length-twice-short-first | origin · origin:GET /r/length-twice-short-first |
| 691 | response status-line-alone, by `use` | fail · origin:GET /r/status-line-alone | fail · origin:GET /r/status-line-alone |
| 692 | response 302-no-length, via http_proxy | `"p1" println\n` · p1:GET http://127.0.0.1:O/r/302-no-length | `"p1" println\n` · p1:GET http://127.0.0.1:O/r/302-no-length |
| 693 | response chunked-twice, via http_proxy | `"p1" println\n` · p1:GET http://127.0.0.1:O/r/chunked-twice | fail · p1:GET http://127.0.0.1:O/r/chunked-twice |
| 694 | response chunked, via http_proxy | `"p1" println\n` · p1:GET http://127.0.0.1:O/r/chunked | `"p1" println\n` · p1:GET http://127.0.0.1:O/r/chunked |
| 695 | no_proxy=127.0.0.0/, 116 zeros, 8: 127 bytes | origin · origin:GET /x | origin · origin:GET /x |
| 696 | no_proxy=127.0.0.0/, 117 zeros, 8: 128 bytes | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 697 | no_proxy=127.0.0.1/ and 117 x: 127 bytes | origin · origin:GET /x | origin · origin:GET /x |
| 698 | no_proxy=127.0.0.1/ and 118 x: 128 bytes | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 699 | no_proxy=a name of 200 bytes, then 127.0.0.1 | origin · origin:GET /x | origin · origin:GET /x |
| 700 | no_proxy=127.0.0.1, then a name of 200 bytes | origin · origin:GET /x | origin · origin:GET /x |
| 701 | no_proxy=127.0.0.1/ and 118 x, then 127.0.0.1 | origin · origin:GET /x | origin · origin:GET /x |
| 702 | host x.bund2.invalid, no_proxy=a name of 200 bytes ending in it | p1 · p1:GET http://x.bund2.invalid:O/x | p1 · p1:GET http://x.bund2.invalid:O/x |
| 703 | host of 200 bytes, no_proxy=its last 130 | fail · - | fail · - |
| 704 | host [::1], no_proxy=::1 after 127 zeros | p1 · p1:GET http://[::1]:O/x | p1 · p1:GET http://[::1]:O/x |
| 705 | proxy http:/127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 706 | proxy http:///127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 707 | proxy http:////127.0.0.1:P1 | fail · - | fail · - |
| 708 | proxy HTTP:/127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 709 | proxy 127.0.0.1:P1/a://b | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 710 | proxy localhost:P1/a://b | p1 · p1:GET http://127.0.0.1:O/x | p1 · p1:GET http://127.0.0.1:O/x |
| 711 | proxy x://127.0.0.1:P1 | fail · - | fail · - |
| 712 | proxy x:/127.0.0.1:P1 | fail · - | fail · - |
| 713 | proxy socks5:/127.0.0.1:P1 | fail · p1:SOCKS5 | fail · - |
| 714 | proxy http+x://127.0.0.1:P1 | fail · - | fail · - |
| 715 | proxy 1http://127.0.0.1:P1 | fail · - | fail · - |
| 716 | proxy http:/u:p@127.0.0.1:P1 | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p] | p1 · p1:GET http://127.0.0.1:O/x [proxy-auth u:p] |
| 717 | proxy http:/127.0.0.1 | p1080 · p1080:GET http://127.0.0.1:O/x | p1080 · p1080:GET http://127.0.0.1:O/x |
| 718 | proxy http:\\127.0.0.1:P1 | fail · - | fail · - |
| 719 | proxy http://127.0.0.1:P1<newline> | fail · - | fail · - |
| 720 | proxy <tab>http://127.0.0.1:P1 | fail · - | fail · - |
| 721 | `bund.eval-file` of /abs/d/t.bund | file · - | file · - |
| 722 | `bund.eval-file` of d/t.bund | fail · - | fail · - |
| 723 | `bund.eval-file` of /abs/lib%20file.bund | file · - | file · - |
| 724 | `bund.eval-file` of /abs/lib file.bund | fail · - | file · - |
| 725 | `bund.eval-file` of /abs/d/link/../t.bund | file · - | file · - |
| 726 | `bund.eval-file` of /abs/d/t.bund?x | file · - | file · - |
| 727 | `bund.eval-file` of /abs/d/t.bund#f | file · - | file · - |
| 728 | `bund.eval-file` of /abs/d/link/%2e%2e/t.bund | linked · - | linked · - |
| 729 | `bund.eval-file` of localhost/abs/d/t.bund | file · - | file · - |
| 730 | `bund.eval-file` of //abs/d/t.bund | file · - | file · - |
| 731 | `bund.eval-file` of ///abs/d/t.bund | file · - | file · - |
| 732 | `bund.eval-file` of /abs/d/nosuch.bund | fail · - | fail · - |
| 733 | `bund.eval-file` of  | fail · - | fail · - |
| 734 | a request of 1,048,575 bytes | fail · origin:other GET /aaaaaaaaaaaaaaaaaaa | body · origin:GET /aaaaaaaaaaaaaaaaaaa… (1048508 bytes) |
| 735 | a request of 1,048,575 bytes, via http_proxy | fail · p1:other GET http://127.0.0.1:O/a | `"p1" println\n` · p1:GET http://127.0.0.1:O/a… (1048478 bytes) |
| 736 | a request of 1,048,576 bytes | fail · origin:silent | fail · origin:silent |
| 737 | a request of 1,048,576 bytes, via http_proxy | fail · p1:silent | fail · p1:silent |
| 738 | an `http:` URL of 7,999,999 bytes | fail · origin:silent | fail · origin:silent |
| 739 | an `http:` URL of 8,000,000 bytes | fail · - | fail · - |
| 740 | the `file` word, a URL of 8,000,000 bytes | file · - | file · - |
| 741 | the `file` word, a URL of 8,000,001 bytes | fail · - | fail · - |
