# tracewing in Korean. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 = … { $count }줄 { $closer }

# … {size} more
ta8dc6eab = … { $size } 더 있음

# Unfold all
t3849d44e = 모두 펼치기

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
       *[other] { $size } · { $count }줄
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · { $percent }% 읽는 중

# Appearance: {now}. Click for {next}.
t332cfa6d = 모양: { $now }. 클릭하면 { $next } 모드로 바뀝니다.

# Method
t98a2c750 = 메서드

# Status
t005ef20f = 상태

# App
t81f51acc = 앱

# Host
t302af18f = 호스트

# Path
teb66e456 = 경로

# Operation
t218e02a6 = 오퍼레이션

# Size
ta6478e7c = 크기

# Time
tdfe4e404 = 시간

# Number
t61e9eac0 = 번호

# "{method}" is not a method
tacdcf5cb = "{ $method }"은(는) 메서드가 아닙니다

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = 요청 본문이 온전히 보관되지 않아 다시 보낼 수 없습니다

# name: value, one a line
t4df37bea = 이름: 값, 한 줄에 하나씩

# No body
tfa4c2902 = 본문 없음

# No flow is chosen
t550b0a98 = 선택한 플로우가 없습니다

# This flow: {e}
t296762f8 = 이 플로우: { $e }

# Request
t82454152 = 요청

# Headers
t334f5989 = 헤더

# Body
t428a1095 = 본문

# Sending…
te946e7bf = 보내는 중…

# Send
tf28e14cf = 보내기

# New
t8b242db1 = 새로 만들기

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … { $count }바이트 더 있음: 트래픽의 플로우에 전부 있습니다.

# Response
t55a982fe = 응답

# Compose
t18049f93 = 작성

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = 직접 작성하거나 다시 보내는 요청입니다. tracewing을 거치므로 규칙이 적용되고 트래픽에도 표시됩니다.

# {version} both sides
t961a5ea0 = 양쪽 모두 { $version }

# {client} to the proxy, {server} to the server
t3d237646 = 프록시까지 { $client }, 서버까지 { $server }

# Queued
t9e437c74 = 대기열

# Connect
tdb89a899 = 연결

# Waiting
tc9450e48 = 대기

# Download
t6964bb19 = 다운로드

# No file picker: {e}
t6935fbd3 = 파일 선택기를 열 수 없습니다: { $e }

# Writing the HAR…
t725a0e89 = HAR 저장 중…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
       *[other] 플로우 { $n }개를 { $path }에 저장했습니다
    }

# The HAR was not written: {e}
tef8ec569 = HAR을 저장하지 못했습니다: { $e }

# Open
t538b10e9 = 열기

# Reading {path}…
tc0e7c8f4 = { $path } 읽는 중…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
       *[other] , 항목 { $n }개 제외
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
       *[other] { $format }에서 플로우 { $n }개를 읽었습니다
    }

# The file was not read: {e}
t7f49510f = 파일을 읽지 못했습니다: { $e }

# it is no longer kept
ta7b4c380 = 더 이상 보관되어 있지 않습니다

# {size} saved to {path}
tc6932467 = { $path }에 저장했습니다 ({ $size })

# The body was not saved: {e}
t189900b4 = 본문을 저장하지 못했습니다: { $e }

# Fiddler session
t9f5c89a3 = Fiddler 세션

# Charles session
tb4f56f2b = Charles 세션

# All
t75afe364 = 전체

# Errors
te9ed60a6 = 오류

# Media
td10fcaa3 = 미디어

# Rule: {rule}
teca2074f = 규칙: { $rule }

# Events
t003420a4 = 이벤트

# JSON lines
te8ce2db0 = JSON 줄

# Not decrypted
t8090a1ba = 복호화 안 됨

# open
td35ec4c9 = 열림

# Reading the request…
t51c96267 = 요청을 읽는 중…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = 요청 본문이 저장되지 않아 쿼리를 보여줄 수 없습니다.

# An operation without a name
t38c4d30b = 이름 없는 오퍼레이션

# {n} of {count}: {name}
t53045460 = { $count }개 중 { $n }: { $name }

# Query
t9e89e783 = 쿼리

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Persisted query: 텍스트는 서버가 가지고 있고, 요청은 해시만 보냅니다.

# Variables
t7996f062 = 변수

# Cut
t58e25863 = 잘라내기

# Copy
t658f3664 = 복사

# Paste
tb34a0288 = 붙여넣기

# Select all
t6af21072 = 모두 선택

# This flow is gone.
t3343999e = 이 플로우는 더 이상 없습니다.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = 이 호스트는 복호화되지 않으므로 보내는 내용을 볼 수 없습니다.

# Still coming: it shows once it ends.
t42be95fb = 아직 오는 중입니다. 끝나면 보입니다.

# No body.
t31e4d044 = 본문이 없습니다.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = 이 본문은 보관되지 않았습니다: 디스크가 거의 가득 찼거나, 기록 대기 중인 데이터가 너무 많았습니다.

# This body was not kept.
te46616b5 = 이 본문은 보관되지 않았습니다.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = 이 본문은 더 이상 없습니다: 가장 오래된 플로우와 함께 삭제되었거나 읽을 수 없습니다.

# Cannot open this body: {e}
t620a039b = 이 본문을 열 수 없습니다: { $e }

# Inspector
t4def410e = 인스펙터

# Choose a flow to see it here
tc7be91e8 = 플로우를 선택하면 여기에 표시됩니다

# Not decrypted. New connections to this host go through unread.
ta05ec37a = 복호화 안 됨. 이 호스트로의 새 연결은 내용을 읽지 않고 통과합니다.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = 이 호스트에서 앱이 실패한다면 앱이 인증서를 고정(피닝)하고 있을 수 있습니다.

# Make a rule
t0ac0e062 = 규칙 만들기

# Decrypt this host
tb7367e2c = 이 호스트 복호화

# Do not decrypt this host
t5fa113f6 = 이 호스트 복호화 안 함

# Request and response
t04e060e6 = 요청과 응답

# Request and response one at a time
t70397119 = 요청과 응답을 하나씩

# Request and response side by side
t3d35ce69 = 요청과 응답을 나란히

# Request above the response
t83c63123 = 요청을 응답 위에

# Text
t3e142d5e = 텍스트

# Picture · {size}
te97ed633 = 이미지 · { $size }

# Messages {count}
te61b841a = 메시지 { $count }

# Headers {headers}
t04b0146d = 헤더 { $headers }

# Cookies {cookies}
td58eb84b = 쿠키 { $cookies }

# Query {query}
t4756f9f5 = 쿼리 { $query }

# Form {form}
tfa6e7203 = 폼 { $form }

# Connection
t09ea4ff9 = 연결

# Save body
te2c44424 = 본문 저장

# Send again
tf5910557 = 다시 보내기

# Edit and send
tf837e542 = 편집 후 보내기

# Copied
t8dc21305 = 복사됨

# Copy as cURL
t866d1e3e = cURL로 복사

# Copy as…
t9c1ad448 = 다른 형식으로 복사…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = 이 이미지를 표시할 수 없습니다. 본문을 저장해서 확인하세요.

# Opening…
tc40f2321 = 여는 중…

# No response
t8c2c37b1 = 응답 없음

# Tunnel, not decrypted
tce693254 = 터널, 복호화 안 됨

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = 이전에 만들어진 연결 { $id } 사용: 연결에 걸린 시간이 없습니다.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = 찾을 수 없음: 종료되었거나 다른 사용자의 것일 수 있습니다

# On another device
t2b964a37 = 다른 기기에서

# CLIENT
t6d3cd99e = 클라이언트

# Device
tbdcbb413 = 기기

# Program
tc4253b6b = 프로그램

# Address
t700560f3 = 주소

# SERVER
te3b27612 = 서버

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = 이 연결의 정보는 더 이상 보관되어 있지 않습니다: 오래된 플로우와 함께 삭제되었습니다.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = 복호화 안 됨: tracewing이 내용을 보지 않고 그대로 전달했습니다.

# No connection was made.
tf348eb92 = 연결이 만들어지지 않았습니다.

# None: it was answered without one.
tac71808e = 없음: 연결 없이 응답했습니다.

# {id}, given up during TLS
t0ec66a8c = { $id }, TLS 중 중단됨

# {id}, made for this flow
t7fe32cfc = { $id }, 이 플로우를 위해 생성됨

# {id}, made before and used again
ta904c078 = { $id }, 이전에 생성되어 재사용됨

# Server
t6d7302f2 = 서버

# None: plain HTTP
t05606349 = 없음: 일반 HTTP

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = 서버가 인증서를 제시하지 않았습니다

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = 서버가 제시한 인증서 (서버 자신의 인증서가 먼저)

# NO CERTIFICATES
te6f31dce = 인증서 없음

# CERTIFICATE
t1c902790 = 인증서

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
       *[other] 인증서 { $n }개 (서버의 인증서가 먼저)
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing이 이 서버와의 통신을 계속하지 않았습니다: { $error }. 앱이 회사 CA 등 다른 방법으로 이 서버를 신뢰한다면 “이 호스트 복호화 안 함”을 선택하세요. 그러면 앱이 직접 서버를 확인합니다.

# Cipher
t5b3bed6a = 암호 스위트

# none
tada7afdb = 없음

# not valid until {when}
t0a14b2d8 = { $when }까지 유효하지 않음

# expired {when}
t932538c2 = { $when } 만료됨

# expires {when}
t679bcb51 = { $when } 만료

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
       *[other] { $names } 외 { $n }개
    }

# Issued by {issuer}
t15a1ce5a = 발급자 { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = 유효 기간 { $from } ~ { $to } · { $when }

# Names {names}
tda75e459 = 이름 { $names }

# Serial {serial}
td1f0d98b = 일련번호 { $serial }

# These headers were not kept.
tb8739fec = 이 헤더는 보관되지 않았습니다.

# No headers.
tf3dacb5e = 헤더가 없습니다.

# File {filename} · {kind} · {size}
t6dbd239f = 파일 { $filename } · { $kind } · { $size }

# no type
t902a6ece = 유형 없음

# No cookies were sent.
t58e85ab2 = 보낸 쿠키가 없습니다.

# No cookies were set.
tf57dbabc = 설정된 쿠키가 없습니다.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · 서명 확인 안 함

# JWT · signature not checked
t05f8b533 = JWT · 서명 확인 안 함

# Header
t114f9980 = 헤더

# Payload
ta0639745 = 페이로드

# Expires
tc7aa0523 = 만료

# Issued
tb1835ecc = 발급

# Not before
td10dacd9 = 유효 시작

# in {amount}
tfda57c3e = { $amount } 후

# {amount} ago
t5f968014 = { $amount } 전

# Light
t8ad2f8cf = 라이트

# Dark
t45570065 = 다크

# System
t8f3184bc = 시스템

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
       *[other] { $count }개 청크로 스트리밍됨
    }

# stopped: {stop}
tfa48fb30 = 중지: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = 입력 { $input } · 출력 { $output } 토큰

# Reading the call…
t33b52a3c = 호출 읽는 중…

# Answer
tf94ea0ff = 답변

# Thinking: {thinking}
t737ece33 = 생각: { $thinking }

# Calls {name}{id}
t017882e4 = { $name }{ $id } 호출

# Nothing yet.
td9b71d16 = 아직 아무것도 없습니다.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
       *[other] { $text }… 외 { $count }자
    }

# Copy the answer
t69975ed7 = 답변 복사

# Asked
t4e3af9c3 = 질문

# Tools offered: {tools}
tb871bcf4 = 제공된 도구: { $tools }

# Not kept
tba30b5fa = 보관 안 됨

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = 이 메시지는 보관되지 않았습니다: 대기 중인 데이터가 너무 많았거나 디스크가 거의 가득 찼습니다.

# This message is empty.
tdb0ec01f = 이 메시지는 비어 있습니다.

# No messages yet
t51e7624c = 아직 메시지가 없습니다

# {said} kept, of {size}
t3558745a = { $size } 중 { $said } 보관됨

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text }은(는) 정수가 아닙니다

# Latency
t23a45ef9 = 지연 시간

# Down
ta4995035 = 다운로드

# Up
t42f35940 = 업로드

# Loss
t69db303a = 패킷 손실

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = 느리게 하려면 지연 시간, 속도 또는 패킷 손실을 입력하세요

# Latency, ms
tefe18ab9 = 지연 시간(ms)

# Down, kbit/s
t08c4ae33 = 다운로드(kbit/s)

# Up, kbit/s
td55cfe9a = 업로드(kbit/s)

# Loss, %
tea4af439 = 패킷 손실(%)

# Only these hosts, like *.example.com; all if empty
t77a1f991 = 이 호스트에만 적용(예: *.example.com). 비워 두면 전체

# Not kept for next time: {e}
tdce4d67c = 다음에 쓸 수 있도록 저장하지 못했습니다: { $e }

# Slow network
td51ddfce = 느린 네트워크

# Off: traffic goes as fast as it can
tf0f34db0 = 끔: 트래픽이 최대 속도로 전송됩니다

# Custom
t9525b78e = 사용자 지정

# Only these hosts
tfe16992b = 이 호스트만

# Apply
tca77bbdb = 적용

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = 다음 요청부터 적용됩니다. 지연 시간은 왕복마다 더해집니다. 속도는 느려지는 모든 플로우가 함께 쓰는 하나의 회선입니다. 손실된 패킷은 TCP처럼 재전송을 기다립니다. 그 외 호스트는 느려지지 않습니다.

# Header line {line} has no ":"
t891b076d = 헤더 { $line }번째 줄에 ":"이 없습니다

# Header line {line} has no name
te3247b96 = 헤더 { $line }번째 줄에 이름이 없습니다

# URL
t7569633e = URL

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
       *[other] 텍스트가 아닌 { $n }바이트: 받은 그대로 전달됩니다.
    }

# Nothing is paused
t8cf68b4c = 일시 정지된 항목이 없습니다

# "{status}" is not a status
t40a529af = "{ $status }"은(는) 상태 코드가 아닙니다

# REQUEST
t1f3f6dd2 = 요청

# RESPONSE
tc85f77de = 응답

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = 일시 정지된 항목이 없습니다. 여기서 트래픽을 멈추려면 규칙 화면에서 일시 정지 규칙을 추가하세요.

# for {method} {url}
t093a86f6 = { $method } { $url } 대상

# The request, before the server gets it
tc840fed6 = 서버가 받기 전의 요청

# The response, before the app gets it
t6683d72a = 앱이 받기 전의 응답

# Send as it came
tb593e730 = 그대로 보내기

# Drop
t108524ac = 버리기

# Paused
tde281feb = 일시 정지됨

# Nothing is waiting.
tea613769 = 대기 중인 항목이 없습니다.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
       *[other] { $n }개가 전송을 기다리고 있습니다.
    }

# Send all as they came
tdce25640 = 모두 그대로 보내기

# Block
tbd074002 = 차단

# Answer from a file
td8edab50 = 파일로 응답

# Send to another host
te3cd85d7 = 다른 호스트로 보내기

# Set a request header
tfad4aaaa = 요청 헤더 설정

# Set a response header
tcbbc20d0 = 응답 헤더 설정

# Remove a request header
t5228dd84 = 요청 헤더 제거

# Remove a response header
teeb5735a = 응답 헤더 제거

# Set a query parameter
t73d9419b = 쿼리 매개변수 설정

# Remove a query parameter
t12633da9 = 쿼리 매개변수 제거

# Change the request body
t41101201 = 요청 본문 변경

# Change the response body
tdd259b73 = 응답 본문 변경

# Change the status
t4bbdfba8 = 상태 변경

# Pause the request
t40ecc39b = 요청 일시 정지

# Pause the response
t18a34799 = 응답 일시 정지

# Status to answer with, like 403
t8f3ebb6c = 응답할 상태 코드(예: 403)

# Full path of a file or folder
t9fb9ae4c = 파일 또는 폴더의 전체 경로

# Where to, like http://localhost:3000
tdfedd004 = 보낼 곳(예: http://localhost:3000)

# Header's name
t80793085 = 헤더 이름

# Its value
t43968778 = 값

# Parameter's name
t313c8e35 = 매개변수 이름

# Text to find, or empty for the whole body
t8255fc87 = 찾을 텍스트, 비우면 본문 전체

# What to put in its place
t03671fd5 = 대신 넣을 내용

# Status the client gets, like 500
t5e4d3088 = 클라이언트가 받을 상태, 예: 500

# "{first}" is not a status
td43128d3 = "{ $first }"은(는) 상태 코드가 아닙니다

# Block with {status}
t1727e984 = { $status } 응답으로 차단

# Answer from {path}
taefc08fa = { $path } 파일로 응답

# Send to {to}
t5e256873 = { $to } 주소로 보내기

# Request: {header}: {value}
taa230766 = 요청: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = 응답: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = 요청: { $header } 없음

# Response: no {header}
t23960a2e = 응답: { $header } 없음

# Query: {name}={value}
t0ee6c1c0 = 쿼리: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = 쿼리: { $name } 없음

# Status {status}
tad048e53 = 상태 { $status }

# Request body: {replace}
te4856c36 = 요청 본문: { $replace }

# Response body: {replace}
t8600dc42 = 응답 본문: { $replace }

# Request body: {find} → {replace}
taec471a9 = 요청 본문: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = 응답 본문: { $find } → { $replace }

# Name
t0fe07306 = 이름

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL(예: api.myapp.dev/v1/*)

# Method, or empty for all
tb2b869ac = 메서드(비워 두면 전체)

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
       *[other] 규칙 { $n }개를 { $path }에 저장했습니다
    }

# The rules were not written: {e}
te7ac257f = 규칙을 저장하지 못했습니다: { $e }

# Import
t573e0af4 = 가져오기

# {path} could not be read: {e}
t32a3ce94 = { $path } 파일을 읽을 수 없습니다: { $e }

# Nothing was imported: {e}
tb9f7257a = 아무것도 가져오지 못했습니다: { $e }

# Edit
tc2c76cb1 = 편집

# Delete
t5797ea6a = 삭제

# Change this rule
t9160f393 = 이 규칙 변경

# A new rule
t55a722a0 = 새 규칙

# It waits under Paused until you send it on, changed or not.
tca566a57 = 변경 여부와 관계없이, 보낼 때까지 일시 정지됨 화면에서 기다립니다.

# Save
t4d2d5d68 = 저장

# Add
t9dc3aa14 = 추가

# Cancel
t35afca3b = 취소

# Rules
t428b2d80 = 규칙

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = 규칙은 다음 요청부터, 기록이 일시 중지된 동안에도 적용됩니다. 위에서 아래로: 처음 맞는 차단, 파일, 호스트 규칙이 이기고, 맞는 다른 규칙은 모두 순서대로 적용됩니다. 본문이나 상태를 바꾸는 규칙은 서버가 보낸 내용을 바꿉니다.

# Import rules…
t5edc205d = 규칙 가져오기…

# Export rules…
t6e09ca20 = 규칙 내보내기…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
       *[other] 규칙 { $n }개 추가됨
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
       *[other] { $n }개는 이미 있음
    }

# left out: {why}
ta81e95e2 = 제외됨: { $why }

# Copy all
t6aef5c37 = 모두 복사

# Too large to copy all
t2fe8ff9f = 너무 커서 모두 복사할 수 없음

# No flows
t6dc463eb = 플로우 없음

# Name it to keep it
t52c1a402 = 이름을 지정하면 보관됩니다

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · 다른 버전에서 작성되어 열 수 없음

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
       *[other] 플로우 { $count }개 · { $size }
    }

# Recording now
t3cb3e6fc = 지금 기록 중

# Export HAR
t4d6458cc = HAR 내보내기

# Delete for good?
ta7737f13 = 영구적으로 삭제할까요?

# Sessions
tb69a8aec = 세션

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = 이름이 있는 세션은 삭제할 때까지 보관됩니다. 나머지는 최신 { $kept }개만 남습니다.

# Working…
td5507854 = 작업 중…

# Open a session file
tae8622fc = 세션 파일 열기

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, Fiddler .saz 또는 Charles .chlsj

# Exported HAR files leave out tokens and cookies
td4a2f543 = 내보낸 HAR 파일에는 토큰과 쿠키가 포함되지 않습니다

# Other devices
t9e7d8362 = 다른 기기

# Let other devices use the proxy
t13e626b3 = 다른 기기에서 프록시 사용 허용

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust. Takes effect when tracewing starts again.
t2e14229d = 네트워크에 있는 휴대폰과 다른 컴퓨터가 트래픽을 여기로 보낼 수 있게 됩니다. 같은 네트워크의 다른 사람도 마찬가지이므로, 신뢰할 수 없는 네트워크에서는 꺼 두세요. tracewing을 다시 시작하면 적용됩니다.

# Servers
tcd0cf113 = 서버

# Trust what this computer trusts
tfea2d91b = 이 컴퓨터가 신뢰하는 인증서 신뢰

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = 이 컴퓨터에 설치된 인증 기관(예: 회사 자체 인증 기관)의 인증서를 가진 서버에는 이 컴퓨터의 브라우저와 같은 방식으로 연결합니다. 끄면 공인 인증 기관만 신뢰합니다. tracewing을 다시 시작하면 적용됩니다.

# AI tools
t294d7ea4 = AI 도구

# Let AI tools see tokens and cookies
te1d26d3c = AI 도구가 토큰과 쿠키를 볼 수 있도록 허용

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = AI 도구는 tracewing의 MCP 서버를 통해 기록된 트래픽을 읽습니다. 끄면 Authorization, Cookie, Set-Cookie 값은 전달되지 않습니다. 즉시 적용됩니다.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = 앱이 HTTP/3를 쓰지 않게 하기

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them. Takes effect when tracewing starts again.
t2aa7be0d = HTTP/3는 UDP를 사용하는데 UDP를 전달하는 프록시는 없으므로, HTTP/3를 쓰는 앱은 tracewing을 우회합니다. 이 설정은 서버 응답에서 HTTP/3 제안을 제거하므로 앱은 HTTP/2를 계속 사용합니다. 플로우에는 서버가 보낸 그대로의 제안이 표시됩니다. tracewing을 다시 시작하면 적용됩니다.

# Size of a session
ta20fa102 = 세션 크기

# Past this, the oldest flows are dropped.
td2914f46 = 이 크기를 넘으면 가장 오래된 플로우부터 삭제됩니다.

# Room to leave on the disk
t8674a3b3 = 디스크에 남길 여유 공간

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = 여유 공간이 이보다 적으면 플로우는 목록에 계속 표시되지만 본문은 보관되지 않습니다.

# Earlier sessions to keep
td356194a = 보관할 이전 세션 수

# Older ones are deleted when a new session starts.
t66556513 = 새 세션이 시작되면 그보다 오래된 세션은 삭제됩니다.

# Bodies
tdb4d0409 = 본문

# Most kept of one body
t777508dd = 본문 하나의 최대 보관 크기

# It still reaches the client whole.
t386fee28 = 클라이언트에는 여전히 전체가 전달됩니다.

# Most unpacked for viewing
tc3fe2efd = 보기용 최대 압축 해제 크기

# Stops a small download that unpacks to many GB.
tf9c49515 = 압축을 풀면 수 GB가 되는 작은 다운로드를 막습니다.

# Small body size
ta8996af1 = 작은 본문 크기

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = 이보다 작은 본문은 메모리에서 기다렸다가 하나의 파일을 함께 씁니다. 더 큰 본문은 도착하는 대로 각자의 파일에 기록됩니다.

# Compress plain text on disk
tee503c2e = 디스크의 일반 텍스트 압축

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = 공간을 절반 정도만 차지하며, 트래픽이 너무 빠르면 건너뜁니다.

# Compression level
td386c9ef = 압축 수준

# 1 to 19. Higher is smaller and slower.
td42a987d = 1~19. 높을수록 작아지고 느려집니다.

# Search
tc646a2c9 = 검색

# Search URLs
t0c3e0b3d = URL 검색

# Finds any part of a URL as you type. Costs little.
t3f22d666 = 입력하는 동안 URL의 어느 부분이든 찾습니다. 부담이 적습니다.

# Search bodies
t8eec0293 = 본문 검색

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = 본문에서 단어를 찾습니다. 본문만큼의 디스크 공간을 쓸 수 있습니다.

# Searched part of each body
tab293b21 = 본문별 검색 범위

# Words past this are not found. More takes more disk.
t0bf65f4a = 이보다 뒤에 있는 단어는 찾지 못합니다. 크게 할수록 디스크를 더 씁니다.

# Under heavy traffic
ta99888cb = 트래픽이 많을 때

# Memory for bodies waiting to be written
t7d6ff799 = 기록 대기 중인 본문용 메모리

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = 이를 넘으면 플로우는 본문 없이 목록에 표시됩니다. 트래픽이 지연되지는 않습니다.

# Memory for large bodies waiting to be written
t07b9de08 = 기록 대기 중인 큰 본문용 메모리

# Past this, large bodies stop being kept.
tfe61a4f3 = 이를 넘으면 큰 본문은 보관되지 않습니다.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }"은(는) 정수가 아닙니다

# {key}: must not be negative
t9c4debaa = { $key }: 음수일 수 없습니다

# Saved. The limits apply the next time tracewing starts.
tc8bd2065 = 저장했습니다. 한도는 다음에 tracewing을 시작할 때 적용됩니다.

# Settings
t4b058728 = 설정

# Back to defaults
ta3162a13 = 기본값으로 되돌리기

# Hosts not to decrypt
tb457f0b3 = 복호화하지 않을 호스트

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies as soon as it is saved.
t7647c310 = 한 줄에 하나씩: 호스트, 또는 그 호스트와 모든 하위 호스트를 뜻하는 *.example.com. 이 호스트들의 트래픽은 내용을 읽지 않고 통과합니다. 이 목록은 저장하는 즉시 적용됩니다.

# Language
t9a73db9b = 언어

# System ({system_name})
tb5c7a1d4 = 시스템({ $system_name })

# Applies at once.
tefb7a0f9 = 즉시 적용됩니다.

# Your browser
tb5a48dd2 = 브라우저

# Its private key is in the system keychain.
t62a47808 = 개인 키는 시스템 키체인에 있습니다.

# Its private key is in ca.key, which only you can read.
td9c6165d = 개인 키는 ca.key에 있으며, 본인만 읽을 수 있습니다.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = 개인 키는 ca.key에 있으며, 본인만 읽을 수 있습니다. 시스템 키체인에 저장할 수 없었기 때문입니다: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = 페이지를 여세요. 브라우저가 인증서 경고를 표시하거나 페이지를 찾지 못하면, 해당 브라우저에서 위 단계 중 하나가 완료되지 않은 것입니다.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name }에서 tracewing을 거쳐 연결되며 tracewing을 신뢰합니다.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name }에서 tracewing을 거쳐 연결되었지만 HTTPS 없이 페이지를 열었습니다. https://로 다시 여세요.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings, then start tracewing again.
t9f95b7b8 = 휴대폰이나 다른 컴퓨터도 트래픽을 여기로 보낼 수 있습니다. 설정에서 “다른 기기에서 프록시 사용 허용”을 켠 다음 tracewing을 다시 시작하세요.

# this computer's address, port {port}
t2188b3f4 = 이 컴퓨터의 주소, 포트 { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = 기기의 Wi-Fi 프록시를 설정합니다: { $addr }. 그런 다음 그 기기에서 http://{ $host } 페이지를 열고 인증서를 받아 신뢰하도록 설정합니다. 그 기기의 플로우에는 기기 주소가 표시되며, device:로 필터링할 수 있습니다. 연결 탭에서 그 플로우 중 하나를 통해 기기 이름을 지정하세요.

# Looking for it…
t50ee7452 = 찾는 중…

# Installing. The system may ask you to confirm.
ta41c38ec = 설치 중입니다. 시스템에서 확인을 요청할 수 있습니다.

# Removing. The system may ask you to confirm.
t62b22726 = 제거하는 중입니다. 시스템이 확인을 요청할 수 있습니다.

# This computer trusts it.
t27d6bacc = 이 컴퓨터가 인증서를 신뢰합니다.

# Installed in: {places}.
tb5bcc244 = 설치된 위치: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = 명령줄 도구는 인증서를 신뢰하지만, 브라우저는 자체 인증서 저장소를 사용합니다. 그 저장소에 추가될 때까지 브라우저에서 tracewing을 거치는 HTTPS는 실패합니다.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing은 이 컴퓨터 전용으로 만든 인증서로 HTTPS를 읽습니다. 이 인증서를 신뢰할 때까지 tracewing을 거치는 HTTPS는 실패합니다.

# Trust the certificate
t26618699 = 인증서 신뢰하기

# Install
tb50cbd40 = 설치

# Also for command-line tools
t0db8faea = 명령줄 도구에도 적용

# Remove
t21a5901d = 제거

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = 이 컴퓨터의 모든 저장소에서 인증서를 제거할까요? 다시 설치할 때까지 tracewing을 거치는 HTTPS는 실패합니다. 인증서는 tracewing이 보관하므로 나중에 다시 설치할 수 있습니다.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = Linux의 브라우저가 인증서를 신뢰하게 하려면 certutil이 필요합니다. 설치한 다음 여기서 설치를 누르세요:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = Linux의 브라우저가 인증서를 신뢰하게 하려면 certutil이 필요합니다. 배포판의 NSS 도구를 설치한 다음 여기서 설치를 누르세요.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = 이 데스크톱에는 tracewing이 바꿀 수 있는 시스템 프록시 설정이 없습니다. 아래 브라우저 항목에서 브라우저를 따로 열거나 터미널을 여세요.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop }에는 tracewing이 바꿀 수 있는 시스템 프록시 설정이 없습니다. 아래 브라우저 항목에서 브라우저를 따로 열거나 터미널을 여세요.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = 시스템 설정을 사용하는 앱은 이제 { $addr } 주소를 거칩니다. tracewing을 닫거나 비정상 종료되면 원래 설정으로 돌아갑니다.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = 시스템 설정을 사용하는 앱의 경우, 이 컴퓨터의 트래픽을 tracewing으로 보냅니다. 또는 앱에서 직접 { $addr } 주소를 지정하세요.

# Turn on the system proxy
t52b88441 = 시스템 프록시 켜기

# Turn off
tcd03e04b = 끄기

# Turn on
t65a0c5a7 = 켜기

# Check it in your browser
tbed6e83a = 브라우저에서 확인

# Open the test page
te4e36f65 = 테스트 페이지 열기

# Set up
t43c06abc = 시작하기

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = 이 컴퓨터의 HTTPS 트래픽을 보기 위한 세 단계입니다.

# See traffic
t980eee79 = 트래픽 보기

# the browsers' own stores
tddf6836c = 브라우저 자체 저장소

# Trusts the certificate.
tc5bba858 = 인증서를 신뢰합니다.

# Does not trust the certificate yet.
tf2a0b8c5 = 아직 인증서를 신뢰하지 않습니다.

# Looking…
t284579ba = 찾는 중…

# Has no profile yet. Open it once, then look again.
t00e4c88d = 아직 프로필이 없습니다. 한 번 연 다음 다시 찾으세요.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 = 프로필 { $n }개 중 { $holding }개가 인증서를 신뢰합니다.

# Has not opened the test page yet.
t67f9cb49 = 아직 테스트 페이지를 열지 않았습니다.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = tracewing을 거쳐 HTTPS로 테스트 페이지를 열었습니다.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = tracewing을 거쳐 테스트 페이지를 열었지만 HTTPS가 아닙니다.

# Browsers
td580efd4 = 브라우저

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = 브라우저마다 무엇을 신뢰하고 트래픽을 어디로 보낼지 따로 정합니다. 확인을 누르면 그 브라우저에서 테스트 페이지가 열립니다. 따로 열기는 tracewing을 거치는 전용 프로필로 실행하며, 시스템 프록시는 건드리지 않습니다.

# Look again
t98416222 = 다시 찾기

# No browser found on this computer.
t0e46bb34 = 이 컴퓨터에서 브라우저를 찾지 못했습니다.

# Check
t592c8de7 = 확인

# Open on its own
t0da7783c = 따로 열기

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = 이 컴퓨터에서 UDP 포트 443이 차단되어 있어, 앱이 HTTP/3 대신 HTTP/2로 전환하고 tracewing을 거칩니다. tracewing을 닫거나 비정상 종료되면 차단이 해제됩니다.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = 일부 앱은 UDP 기반의 HTTP/3를 사용해 프록시를 우회합니다. 이 컴퓨터에서 UDP 포트 443을 차단하면 이런 앱이 HTTP/2로 전환해 tracewing을 거치게 됩니다. 관리자 암호가 필요하며, tracewing을 닫으면 차단이 해제됩니다. 그동안 UDP 포트 443을 사용하는 VPN은 작동하지 않습니다.

# Apps on HTTP/3
t2cfe5270 = HTTP/3를 쓰는 앱

# Stop blocking
t318358d8 = 차단 해제

# Block HTTP/3
t88231e40 = HTTP/3 차단

# Or one app at a time
t91c63f1c = 또는 앱 하나씩

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = tracewing을 거치는 터미널입니다. 시스템 프록시와 다른 앱은 그대로 둡니다. 브라우저라면 브라우저 항목의 따로 열기가 전용 프로필로 같은 일을 합니다.

# Open a terminal
t4b629c72 = 터미널 열기

# iPhone and iPad
ta550cc97 = iPhone 및 iPad

# Flutter apps
ta04ee397 = Flutter 앱

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. 설정 > Wi-Fi에서 네트워크를 선택한 다음 ‘프록시 구성’ > ‘수동’에서 서버 { $server }, 포트 { $port }를 입력합니다.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. 카메라로 코드를 스캔하거나 Safari에서 { $page } 페이지를 열고 인증서를 받습니다. 다운로드를 허용하세요.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. 설정 > 일반 > VPN 및 기기 관리에서 tracewing CA 프로파일을 선택해 설치합니다.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. 설정 > 일반 > 정보 > 인증서 신뢰 설정에서 tracewing CA에 대해 ‘루트 인증서 전체 신뢰’를 켭니다.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Safari에서 https://{ $host } 페이지를 엽니다. 작동한다고 표시되면, 시스템 설정을 사용하는 앱은 tracewing을 거칩니다.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. 설정 > Wi-Fi에서 네트워크를 편집하고, ‘고급 옵션’에서 프록시를 ‘수동’으로 설정해 호스트 { $host }, 포트 { $port }를 입력합니다.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. 코드를 스캔하거나 브라우저에서 { $page } 페이지를 열고 인증서를 받습니다.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. 설정 > 보안 > 암호화 및 사용자 인증 정보(이름은 제조사마다 다름)에서 ‘인증서 설치’ > ‘CA 인증서’를 선택하고 tracewing.crt를 고릅니다.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Android 7부터 앱은 사용자가 추가한 인증서를 앱이 명시한 경우에만 신뢰합니다. 직접 만든 앱의 디버그 빌드에는 res/xml/network_security_config.xml을 추가합니다:

# and point to it from AndroidManifest.xml:
t02ad844b = 그리고 AndroidManifest.xml에서 이 파일을 가리키게 합니다:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = 다른 앱은 계속 인증서를 거부하며, 그 트래픽은 실패한 플로우로 표시됩니다.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = Flutter의 http와 Dio가 사용하는 Dart의 HttpClient는 휴대폰의 프록시 설정을 따르지 않으며, Android에서는 사용자가 추가한 인증서를 신뢰하지 않습니다. 디버그 빌드에서 다음과 같이 여기로 보내도록 합니다:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback은 모든 인증서를 통과시키므로 디버그 빌드에서만 사용해야 합니다.

# Open settings
t1336b4d6 = 설정 열기

# Phones and other devices
t2e3c591c = 휴대폰 및 다른 기기

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = 이 네트워크의 휴대폰, 태블릿 또는 다른 컴퓨터는 { $proxy } 주소로 tracewing을 거칠 수 있습니다. 코드를 스캔하면 인증서가 있는 페이지가 열립니다. 그 기기의 플로우에는 기기 주소가 표시되고, device:로 필터링할 수 있으며, 연결 탭에서 그 플로우 중 하나를 통해 이름을 지정할 수 있습니다.

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = 설정 완료. 기기가 다시 시작될 때까지 인증서가 시스템 인증서에 포함됩니다.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = 설정 완료. 인증서는 다운로드 폴더에 있습니다. 방금 열린 설정에서 ‘암호화 및 사용자 인증 정보’ > ‘인증서 설치’ > ‘CA 인증서’를 선택하고 tracewing.crt를 고르세요.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = 시스템 인증서만 신뢰하는 앱은 여전히 이 인증서를 거부합니다.

# An emulator.
t14b6b86d = 에뮬레이터입니다.

# On USB.
t67dafc14 = USB로 연결되어 있습니다.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = 기기에서 USB 디버깅 확인 창을 허용한 다음 다시 찾아보세요.

# Still starting. Look again in a moment.
t13f68b64 = 아직 시작 중입니다. 잠시 후 다시 찾아보세요.

# adb says it is {state}.
t2b7d71da = adb에 따르면 상태가 { $state }입니다.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Android 에뮬레이터 또는 USB 디버깅이 켜진 상태로 USB에 연결된 휴대폰에 프록시를 설정하고 인증서를 설치합니다. 트래픽은 ADB를 통해 전달되므로 둘 다 이 네트워크에 있을 필요가 없습니다.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = adb를 찾을 수 없습니다. Android Studio 또는 Android 플랫폼 도구를 설치한 다음 다시 찾아보세요.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = 연결된 기기가 없습니다. 에뮬레이터를 시작하거나 USB 디버깅이 켜진 휴대폰을 연결한 다음 다시 찾아보세요.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = 기기가 트래픽을 여기로 보내도록 설정합니다. 실행 취소하거나 tracewing을 닫으면 기기의 프록시 설정이 다시 해제됩니다.

# Undo
t71fd4acf = 실행 취소

# Android emulators and USB devices
t379e433c = Android 에뮬레이터 및 USB 기기

# Find devices
t54d4af39 = 기기 찾기

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code 및 기타 AI 도구는 tracewing의 MCP 서버를 통해 tracewing이 기록한 플로우를 나열하고 읽을 수 있습니다. 설정에서 허용하지 않으면 토큰과 쿠키는 제외됩니다. Claude Code에서는:

# Elsewhere, in the tool's MCP settings:
tec38833c = 다른 도구에서는 해당 도구의 MCP 설정에서:

# Flows
t4db77db2 = 플로우

# Choose the flow above or below
tecb3adec = 위 또는 아래 플로우 선택

# The first or the last flow
t7e278a00 = 첫 번째 또는 마지막 플로우

# A page up or down
t23b1814b = 한 페이지 위 또는 아래로

# Filter
tf4a9c097 = 필터

# Pause or resume recording
t6e21519a = 기록 일시 정지 또는 재개

# Clear the list
tf2a55d1f = 목록 지우기

# The chosen flow
te0a729de = 선택한 플로우

# Its request
tb3970784 = 해당 요청

# Its response
t221fc168 = 해당 응답

# Send it again
tef7d1944 = 다시 보내기

# Edit it and send it
t6bc99e5c = 편집 후 보내기

# Copy it as cURL
tc99a11a3 = cURL로 복사

# The part before or after the one shown
tdf4221d9 = 보이는 탭의 앞 또는 뒤 탭

# Save the body shown
t20d9b7c0 = 보이는 본문 저장

# Select all of the part shown
tf9168168 = 표시된 부분 모두 선택

# Copy the selected text
t00b090d3 = 선택한 텍스트 복사

# Send it on, as edited
t8c35fa5a = 편집한 대로 보내기

# Send it on as it was
t16f7abb5 = 원래대로 보내기

# Drop it
t4e7f48b9 = 버리기

# The window
t2af18488 = 창

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = 시작하기, 트래픽, 규칙, 일시 정지됨, 작성, 세션, 설정

# Show or hide the sidebar
t67adca04 = 사이드바 표시 또는 숨기기

# Show or hide the inspector
ta5a7b9e9 = 인스펙터 표시 또는 숨기기

# The inspector beside the list or below it
t51e214f7 = 인스펙터를 목록 옆 또는 아래에 표시

# The next or the previous field
ta4b4bd5e = 다음 또는 이전 입력란

# These shortcuts
tbdf3b62b = 이 단축키 목록

# Close them
tf3a4b1ef = 목록 닫기

# Keyboard shortcuts
t31eaddb7 = 키보드 단축키

# Close
tcd86acc3 = 닫기

# The certificate: {e}
t0235648c = 인증서: { $e }

# The terminal
tf542b2f4 = 터미널

# The system proxy: {e}
tc99726ce = 시스템 프록시: { $e }

# Blocking HTTP/3: {e}
t024ec871 = HTTP/3 차단: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx 또는 본문 텍스트

# A name, as iPhone test
t19f89214 = 이름(예: iPhone 테스트)

# The search index: {e}
te3c971c9 = 검색 색인: { $e }

# An earlier session: {name}
t73446a1d = 이전 세션: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = 읽기 전용. 색인이 완성될수록 검색 범위가 넓어집니다. 기록은 계속됩니다.

# Read-only. Recording goes on.
te2f35d9e = 읽기 전용. 기록은 계속됩니다.

# Back to live
t317c0453 = 실시간으로 돌아가기

# {shown} of {total}
tf2cc0563 = { $shown } / { $total }

# Recording
te75cf20c = 기록 중

# System proxy off · {addr}
t7e813d25 = 시스템 프록시 꺼짐 · { $addr }

# System proxy…
t29e7dcd0 = 시스템 프록시…

# System proxy on · {addr}
t319ad6b9 = 시스템 프록시 켜짐 · { $addr }

# Apps
t53d91aad = 앱

# Devices
t27acf320 = 기기

# Hosts
tcf9aefb4 = 호스트

# All hosts
tfc7bfe37 = 모든 호스트

# Hidden
t3c2dfe59 = 숨김

# Newest
t324b6d0b = 최신

# No flow fits this filter
t7eb295f4 = 이 필터에 맞는 플로우가 없습니다

# Nothing yet
tbd00747c = 아직 아무것도 없습니다

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = 위에서 시스템 프록시를 켜거나, 앱에서 { $addr } 주소를 지정하세요. 테스트하려면: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = 목록 옆

# Below the list
t23658daf = 목록 아래

# Drop it on a place · Esc puts it back
t0fa632ba = 원하는 위치에 놓기 · Esc로 되돌리기

# Traffic
t4b589266 = 트래픽

# Certificate trusted on this computer
t7b93f551 = 이 컴퓨터에서 인증서를 신뢰함

# Checking the certificate…
tbf8942c3 = 인증서 확인 중…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = 인증서 설치 중입니다. 시스템에서 확인을 요청할 수 있습니다…

# Removing the certificate; the system may ask you to confirm…
t913da735 = 인증서를 제거하는 중입니다. 시스템이 확인을 요청할 수 있습니다…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = 인증서를 신뢰할 때까지 HTTPS가 실패합니다: 시작하기

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml을 사용하지 않았습니다: { $problem }

# The proxy stopped: {failure}
t04dcff11 = 프록시가 중지되었습니다: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = 디스크가 거의 가득 찼습니다: 본문을 보관하지 않고 있습니다

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
       *[other] 플로우 { $count }개가 기록되지 않음
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
       *[other] 본문이 없는 플로우 { $count }개
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
       *[other] { $size } 이하로 유지하기 위해 가장 오래된 플로우 { $count }개를 삭제함
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
       *[other] 플로우 { $count }개
    }

# Export as HAR
t95b6b766 = HAR로 내보내기

# Shortcuts {keys}
t4ab0ecb4 = 단축키 { $keys }

# Columns
tea156fdc = 열
