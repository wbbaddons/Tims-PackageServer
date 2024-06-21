GET http://localhost:9001
HTTP 200
content-type: text/xml; charset=utf-8
[Asserts]

xpath "//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test']" exists
xpath "//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test']/_:versions/_:version" count == 1
xpath "string(//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test']/_:versions/_:version[@name='1.0.0']/@accessible)" == "true"

xpath "//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test2']" exists
xpath "//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test2']/_:versions/_:version" count == 2
xpath "string(//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test2']/_:versions/_:version[@name='1.0.0']/@accessible)" == "false"
xpath "string(//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test2']/_:versions/_:version[@name='1.0.1']/@accessible)" == "false"

[Captures]
test_1_0_0_download: xpath "string(//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test']/_:versions/_:version/_:file)"
test2_1_0_0_download: xpath "string(//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test2']/_:versions/_:version[@name='1.0.0']/_:file)"
test2_1_0_1_download: xpath "string(//_:section[@name='packages']/_:package[@name='be.bastelstu.packages.test2']/_:versions/_:version[@name='1.0.1']/_:file)"

GET {{ test_1_0_0_download }}
HTTP 200
content-type: application/x-tar

GET {{ test2_1_0_0_download }}
HTTP 401

GET {{ test2_1_0_0_download }}
[BasicAuth]
test: test
HTTP 200
content-type: application/x-tar

GET {{ test2_1_0_1_download }}
[BasicAuth]
test: test
HTTP 401
