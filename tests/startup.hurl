GET http://localhost:9001
[Options]
retry: 15
HTTP 200

GET http://localhost:9001
HTTP 200
[Captures]
etag: header "ETag"

GET http://localhost:9001
if-none-match: {{etag}}
HTTP 304

GET http://localhost:9002/auth.json
[Options]
output: packages/auth.json

GET http://localhost:9001
if-none-match: {{etag}}
[Options]
retry: 15
HTTP 200
