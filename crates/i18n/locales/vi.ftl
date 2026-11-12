# tracewing in Vietnamese. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 = … { $count } dòng { $closer }

# … {size} more
ta8dc6eab = … còn { $size }

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc = { $size } { $format } · { $count } dòng JSON

# The first {count} rows are shown.
t16aa1c6d = Chỉ hiện { $count } dòng đầu tiên.

# Clear what was read
tdd38ff77 = Bỏ phần đã đọc

# Unfold all
t3849d44e = Mở hết

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
       *[other] { $size } · { $count } dòng
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · đang đọc { $percent }%

# {call} could not be put in the body as calldata. Write it as hex, or let tracewing write the body.
t1d03e346 = Không đặt được { $call } vào body dưới dạng calldata. Hãy viết nó thành hex, hoặc để tracewing viết body.

# That is not a call written out.
tc159f8f5 = Đó không phải một lời gọi được viết ra.

# No call named {name} is known here. Write its types with it, as transfer(address,uint256)(…), or add its ABI under Contract ABIs in Settings.
t61568d43 = Ở đây không biết lời gọi nào tên { $name }. Hãy viết kèm kiểu tham số, như transfer(address,uint256)(…), hoặc thêm ABI của nó ở ABI của contract trong Cài đặt.

# {name} is more than one call: {found}. Write the types with it, as {first}(…).
t48b75c8d = { $name } ứng với nhiều lời gọi: { $found }. Hãy viết kèm kiểu tham số, như { $first }(…).

# {types} are not types a call can be written with.
t544218b9 = { $types } không phải kiểu mà một lời gọi có thể dùng.

# The values do not fit {signature}: {why}
t14015b69 = Các giá trị không khớp { $signature }: { $why }

# Appearance: {now}. Click for {next}.
t332cfa6d = Giao diện: { $now }. Bấm để chuyển sang { $next }.

# api.example.com, or *.example.com
tdc6588ac = api.example.com, hoặc *.example.com

# Choose
t73c66160 = Chọn

# Choose the certificate's file.
t6c7ace6e = Hãy chọn file chứng chỉ.

# Say which host it is for.
taf39b2e1 = Hãy cho biết chứng chỉ dùng cho host nào.

# Not used: {why}
t3a0298f3 = Không dùng: { $why }

# Remove
t21a5901d = Gỡ

# Client certificates
te3ce8812 = Chứng chỉ client

# For servers that ask the client for a certificate (mutual TLS): tracewing shows the one given for their host, in the app's place. A PEM certificate and its key without a passphrase; the files stay where they are. Each applies to the next connection.
tcd3f6864 = Cho các server yêu cầu client đưa chứng chỉ (mutual TLS): tracewing đưa chứng chỉ đã gán cho host của server đó, thay cho ứng dụng. Một chứng chỉ PEM và khoá của nó, không có mật khẩu; các file vẫn nằm nguyên chỗ cũ. Mỗi chứng chỉ có hiệu lực từ kết nối kế tiếp.

# None yet.
t786bc275 = Chưa có.

# Certificate…
t164f4d6c = Chứng chỉ…

# none chosen
t23b63bc7 = chưa chọn

# Key…
t7aa250a8 = Khoá…

# in the certificate's file
t13bc7052 = nằm trong file chứng chỉ

# Add
t9dc3aa14 = Thêm

# Method
t98a2c750 = Method

# Status
t005ef20f = Status

# App
t81f51acc = App

# Host
t302af18f = Host

# Path
teb66e456 = Đường dẫn

# Operation
t218e02a6 = Operation

# Size
ta6478e7c = Kích thước

# Time
tdfe4e404 = Thời gian

# Number
t61e9eac0 = Số thứ tự

# The request is in {path}. Give it to your IT team to sign as an intermediate CA, then take in what they send back.
td2c32731 = Yêu cầu đã lưu ở { $path }. Hãy đưa cho bộ phận IT để ký thành CA trung gian, rồi nhập lại thứ họ gửi về.

# Take in
t3118cd2d = Nhập

# Signed by {issuer}: restart tracewing to use it. Nothing needs installing on this company's machines.
tb68eb888 = Đã được { $issuer } ký: hãy khởi động lại tracewing để dùng. Không cần cài gì trên các máy của công ty.

# It cannot be used: {why}
tb4233c11 = Không dùng được: { $why }

# Back to tracewing's own certificate: restart tracewing to use it.
t4a1a1e86 = Đã quay về chứng chỉ của tracewing: hãy khởi động lại tracewing để dùng.

# Now: signed by {issuer}, until {until}.
tcb26a623 = Hiện tại: do { $issuer } ký, đến { $until }.

# Now: tracewing's own, which signs itself, until {until}.
t900843cd = Hiện tại: chứng chỉ riêng của tracewing, tự ký, đến { $until }.

# Certificate signed by your company
t1df2d89b = Chứng chỉ do công ty bạn ký

# Where IT lets no new root certificate in: tracewing makes a request for its CA's key, your IT team signs it as an intermediate CA under the company's root, and tracewing uses that instead. The company's computers trust it already, with nothing to install.
t0489607f = Khi IT không cho cài root mới: tracewing tạo yêu cầu cho khoá CA của nó, IT ký thành CA trung gian dưới root của công ty, và tracewing dùng chứng chỉ đó. Máy trong công ty vốn đã tin nó, không cần cài gì.

# Make the request (CSR)…
t61f8c4ae = Tạo yêu cầu (CSR)…

# Take in the signed certificate…
t2f79e9d7 = Nhập chứng chỉ đã ký…

# Back to tracewing's own
t9c075d1b = Quay về chứng chỉ của tracewing

# A body was not kept, so the two cannot be compared.
t9d6abb2a = Một body không được giữ lại, nên không so sánh được.

# {count} field differs | {count} fields differ
t6a8fff7f = { $count } trường khác nhau

# … {count} line the same | … {count} lines the same
tcc37e7ea = … { $count } dòng giống nhau

# Nothing to compare.
t9f32d942 = Không có gì để so sánh.

# URL
t7569633e = URL

# Protocol
t4a57ebd5 = Giao thức

# Type
td155d06d = Loại

# Sizes
td1a4a19d = Kích thước

# Body
t428a1095 = Body

# Headers
t334f5989 = Header

# Query
t9e89e783 = Query

# Request
t82454152 = Yêu cầu

# Response
t55a982fe = Phản hồi

# Neither has headers kept.
tb43ef094 = Cả hai đều không giữ header.

# Trailers
t7c44fa89 = Trailer

# Neither URL has a query.
t4d8203e0 = Cả hai URL đều không có query.

# Comparing…
ta8565d31 = Đang so sánh…

# No body
tfa4c2902 = Không có body

# {kind}, its first {size} compared
t1d894c39 = { $kind }, đã so sánh { $size } đầu tiên

# Neither has a body.
tf2c47725 = Cả hai đều không có body.

# The bodies are the same.
t85152e01 = Hai body giống nhau.

# {count} field differs. | {count} fields differ.
t0f651f77 = { $count } trường khác nhau.

# The first {count} are listed.
taae62a3e = Chỉ liệt kê { $count } trường đầu tiên.

# {count} line differs. | {count} lines differ.
tf57a3157 = { $count } dòng khác nhau.

# Fields
t3308bd9c = Trường

# Lines
t8a1a00dc = Dòng

# Compare
td57a0a54 = So sánh

# #{a} beside #{b}: what only #{a} has is red, what only #{b} has is green.
t71ce2de9 = #{ $a } cạnh #{ $b }: phần chỉ #{ $a } có tô đỏ, phần chỉ #{ $b } có tô xanh.

# Swap
te519d36e = Đổi chỗ

# Back to Traffic
t644042d2 = Về Traffic

# As agreed
td3a1f5f7 = Theo thoả thuận

# "{method}" is not a method
tacdcf5cb = "{ $method }" không phải là một method

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = body của request không được giữ đầy đủ, nên không gửi lại được

# name: value, one a line
t4df37bea = tên: giá trị, mỗi dòng một header

# No flow is chosen
t550b0a98 = Chưa chọn flow nào

# This flow: {e}
t296762f8 = Flow này: { $e }

# The body is not a {message}: {e}
t3220a10a = Body không phải một { $message }: { $e }

# The body of {bytes} bytes was too large to keep when the window last closed, so this request came back without it.
t781059cd = Body dài { $bytes } byte quá lớn để giữ lại khi đóng cửa sổ lần trước, nên request này trở lại mà không có body.

# This request came back without its body: the body it had was bytes that are not the text shown for them, and writing the text down would have sent something else.
t5e2cb6ff = Request này trở lại mà không có body: body là các byte không phải đoạn text hiển thị cho chúng, và ghi lại text thì lần sau sẽ gửi một thứ khác.

# Protobuf {message}, written as JSON: it is sent as Protobuf.
t83b6cd6e = Protobuf { $message }, viết dưới dạng JSON: khi gửi sẽ là Protobuf.

# Written out in the body and sent as calldata: {calls}
tcaecdf04 = Được viết ra trong body và gửi dưới dạng calldata: { $calls }

# {names} is the client's to work out, so what was written for it did not go. | {names} are the client's to work out, so what was written for them did not go.
te85cbcea =
    { $n ->
       *[other] { $names } do client tự quyết định, nên giá trị viết cho chúng không được gửi đi.
    }

# {version} is how tracewing is asked. What the server is asked with is settled between tracewing and the server, as it is for every other flow.
t4916bc86 = { $version } là phiên bản dùng để hỏi tracewing. Phiên bản dùng để hỏi server do tracewing và server tự thoả thuận, như với mọi flow khác.

# Stopped: no answer was read. What reached the server shows in Traffic.
t5a78825c = Đã dừng: chưa đọc phản hồi nào. Phần đã tới server hiện trong Traffic.

# Ask with
tb5209e72 = Hỏi bằng

# Sending…
te946e7bf = Đang gửi…

# Send
tf28e14cf = Gửi

# Stop
t4b7f7705 = Dừng

# New
t8b242db1 = Mới

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … còn { $count } byte nữa: flow trong Traffic có đầy đủ.

# Body · {message}
taa3cc3ff = Body · { $message }

# Compose
t18049f93 = Soạn request

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Một request do bạn soạn, hoặc gửi lại. Nó đi qua tracewing, nên các quy tắc vẫn áp dụng và nó cũng hiện trong Traffic.

# {version} both sides
t961a5ea0 = { $version } cả hai phía

# {client} to the proxy, {server} to the server
t3d237646 = { $client } tới proxy, { $server } tới server

# Queued
t9e437c74 = Chờ trong hàng

# Connect
tdb89a899 = Kết nối

# Waiting
tc9450e48 = Chờ phản hồi

# Download
t6964bb19 = Tải về

# No file picker: {e}
t6935fbd3 = Không mở được hộp chọn file: { $e }

# Writing the HAR…
t725a0e89 = Đang ghi file HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
       *[other] Đã ghi { $n } flow vào { $path }
    }

# The HAR was not written: {e}
tef8ec569 = Chưa ghi được file HAR: { $e }

# Open
t538b10e9 = Mở

# Reading {path}…
tc0e7c8f4 = Đang đọc { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
       *[other] , bỏ qua { $n } mục
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
       *[other] Đã đọc { $n } flow từ { $format }
    }

# The file was not read: {e}
t7f49510f = Chưa đọc được file: { $e }

# it is no longer kept
ta7b4c380 = nó không còn được giữ

# {size} saved to {path}
tc6932467 = Đã lưu { $size } vào { $path }

# The body was not saved: {e}
t189900b4 = Chưa lưu được body: { $e }

# Fiddler session
t9f5c89a3 = phiên Fiddler

# Charles session
tb4f56f2b = phiên Charles

# All
t75afe364 = Tất cả

# Errors
te9ed60a6 = Lỗi

# Media
td10fcaa3 = Đa phương tiện

# Rule: {rule}
teca2074f = Quy tắc: { $rule }

# Events
t003420a4 = Sự kiện

# JSON lines
te8ce2db0 = Dòng JSON

# Not decrypted
t8090a1ba = Không giải mã

# open
td35ec4c9 = đang mở

# Reading the request…
t51c96267 = Đang đọc request…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = Body của request không được lưu, nên không hiện được query.

# An operation without a name
t38c4d30b = Operation không có tên

# {n} of {count}: {name}
t53045460 = { $n }/{ $count }: { $name }

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Persisted query: server giữ nội dung query, request chỉ gửi mã hash của nó.

# Variables
t7996f062 = Variables

# Cut
t58e25863 = Cắt

# Copy
t658f3664 = Sao chép

# Paste
tb34a0288 = Dán

# Select all
t6af21072 = Chọn tất cả

# This flow is gone.
t3343999e = Flow này không còn nữa.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Host này không được giải mã, nên không xem được những gì nó gửi.

# Still coming: it shows once it ends.
t42be95fb = Vẫn đang tới: body hiện khi kết thúc.

# No body.
t31e4d044 = Không có body.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Body này không được giữ: đĩa gần đầy, hoặc có quá nhiều dữ liệu đang chờ ghi.

# This body was not kept.
te46616b5 = Body này không được giữ.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Body này không còn: nó đã bị xoá cùng các flow cũ nhất, hoặc không đọc được.

# Cannot open this body: {e}
t620a039b = Không mở được body này: { $e }

# Inspector
t4def410e = Chi tiết

# Choose a flow to see it here
tc7be91e8 = Chọn một flow để xem ở đây

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Không giải mã. Các kết nối mới tới host này đi qua mà không bị đọc.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Nếu một app lỗi với host này, có thể app đó ghim chứng chỉ.

# Make a rule
t0ac0e062 = Tạo quy tắc

# Decrypt this host
tb7367e2c = Giải mã host này

# Do not decrypt this host
t5fa113f6 = Không giải mã host này

# Request and response
t04e060e6 = Request và response

# Request and response one at a time
t70397119 = Request và response, từng cái một

# Request and response side by side
t3d35ce69 = Request và response cạnh nhau

# Request above the response
t83c63123 = Request ở trên response

# Table
td70ceb7f = Bảng

# Pretty
t7e09470d = Định dạng

# Raw
ta689aa91 = Nguyên bản

# Picture · {size}
te97ed633 = Hình ảnh · { $size }

# Messages {count}
te61b841a = Tin nhắn { $count }

# Headers {headers}
t04b0146d = Header { $headers }

# Cookies {cookies}
td58eb84b = Cookie { $cookies }

# Query {query}
t4756f9f5 = Query { $query }

# Form {form}
tfa6e7203 = Form { $form }

# Connection
t09ea4ff9 = Kết nối

# Save body
te2c44424 = Lưu body

# Send again
tf5910557 = Gửi lại

# Edit and send
tf837e542 = Sửa rồi gửi

# Copied
t8dc21305 = Đã sao chép

# Copy as cURL
t866d1e3e = Sao chép dạng cURL

# Click another flow…
t59253578 = Bấm vào flow khác…

# Compare with…
t58dfe040 = So sánh với…

# Compare with the call before
tef15286c = So với lần gọi trước

# Copy as…
t9c1ad448 = Sao chép dạng…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Không vẽ được hình này. Hãy lưu body để xem.

# Opening…
tc40f2321 = Đang mở…

# No response
t8c2c37b1 = Không có phản hồi

# Tunnel, not decrypted
tce693254 = Tunnel, không giải mã

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# RPC: {method}
tf8162891 = RPC: { $method }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Đi qua kết nối { $id } đã có từ trước: không mất thời gian kết nối.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Không tìm thấy: có thể nó đã tắt, hoặc thuộc về người dùng khác

# On another device
t2b964a37 = Trên thiết bị khác

# CLIENT
t6d3cd99e = CLIENT

# Device
tbdcbb413 = Thiết bị

# Program
tc4253b6b = Chương trình

# Address
t700560f3 = Địa chỉ

# SERVER
te3b27612 = SERVER

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = Thông tin về kết nối này không còn được giữ: nó đã bị xoá cùng các flow cũ hơn.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Không giải mã: tracewing chuyển tiếp mà không đọc.

# No connection was made.
tf348eb92 = Không có kết nối nào được tạo.

# None: it was answered without one.
tac71808e = Không có: nó được trả lời mà không cần kết nối.

# {id}, given up during TLS
t0ec66a8c = { $id }, bỏ dở trong lúc TLS

# {id}, made for this flow
t7fe32cfc = { $id }, tạo cho flow này

# {id}, made before and used again
ta904c078 = { $id }, đã có từ trước và được dùng lại

# Server
t6d7302f2 = Server

# None: plain HTTP
t05606349 = Không có: HTTP thường

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = SERVER KHÔNG ĐƯA RA CHỨNG CHỈ NÀO

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = NHỮNG GÌ SERVER ĐƯA RA, CỦA CHÍNH NÓ TRƯỚC

# NO CERTIFICATES
te6f31dce = KHÔNG CÓ CHỨNG CHỈ

# CERTIFICATE
t1c902790 = CHỨNG CHỈ

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
       *[other] { $n } CHỨNG CHỈ, CỦA SERVER TRƯỚC
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing không tiếp tục với server này: { $error }. Nếu app tin server theo cách khác, chẳng hạn qua CA của công ty, hãy chọn “Không giải mã host này” và app sẽ tự kiểm tra server.

# Cipher
t5b3bed6a = Bộ mã hoá

# none
tada7afdb = không có

# not valid until {when}
t0a14b2d8 = chưa có hiệu lực tới { $when }

# expired {when}
t932538c2 = đã hết hạn { $when }

# expires {when}
t679bcb51 = hết hạn { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
       *[other] { $names }, và { $n } tên khác
    }

# Issued by {issuer}
t15a1ce5a = Cấp bởi { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Hiệu lực từ { $from } tới { $to } · { $when }

# Names {names}
tda75e459 = Tên { $names }

# Serial {serial}
td1f0d98b = Số sê-ri { $serial }

# These headers were not kept.
tb8739fec = Các header này không được giữ.

# No headers.
tf3dacb5e = Không có header.

# Trailers, sent after the body
t3447e9de = Trailers, gửi sau body

# File {filename} · {kind} · {size}
t6dbd239f = File { $filename } · { $kind } · { $size }

# no type
t902a6ece = không rõ kiểu

# No cookies were sent.
t58e85ab2 = Không có cookie nào được gửi.

# No cookies were set.
tf57dbabc = Không có cookie nào được đặt.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · chưa kiểm tra chữ ký

# JWT · signature not checked
t05f8b533 = JWT · chưa kiểm tra chữ ký

# Header
t114f9980 = Header

# Payload
ta0639745 = Payload

# Light
t8ad2f8cf = Sáng

# Dark
t45570065 = Tối

# System
t8f3184bc = Theo hệ thống

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
       *[other] stream thành { $count } phần
    }

# stopped: {stop}
tfa48fb30 = dừng: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = { $input } vào · { $output } ra token

# Reading the call…
t33b52a3c = Đang đọc lời gọi…

# Answer
tf94ea0ff = Trả lời

# Thinking: {thinking}
t737ece33 = Suy nghĩ: { $thinking }

# Calls {name}{id}
t017882e4 = Gọi { $name }{ $id }

# Nothing yet.
td9b71d16 = Chưa có gì.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
       *[other] { $text }… và { $count } ký tự nữa
    }

# Copy the answer
t69975ed7 = Sao chép câu trả lời

# Asked
t4e3af9c3 = Đã hỏi

# Tools offered: {tools}
tb871bcf4 = Tool được cung cấp: { $tools }

# Not kept
tba30b5fa = Không được giữ

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Tin nhắn này không được giữ: quá nhiều dữ liệu đang chờ, hoặc đĩa gần đầy.

# This message is empty.
tdb0ec01f = Tin nhắn này trống.

# No messages yet
t51e7624c = Chưa có tin nhắn

# {said} kept, of {size}
t3558745a = Giữ { $said } trong { $size }

# The socket has closed.
t8d883a0f = Socket đã đóng.

# Too many are still waiting to go. Try again.
tbea55dfd = Còn quá nhiều message đang chờ gửi. Hãy thử lại.

# To the server
te1ef193a = Tới server

# To the app
t1bcabce4 = Tới ứng dụng

# Layouts
t17809004 = Bố cục

# Keep the panels as they are now under a name, to come back to them.
t2b6aeb69 = Lưu cách sắp xếp các khung hiện tại dưới một tên, để quay lại sau.

# Keep
t555cbe68 = Lưu

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } không phải là số nguyên

# Latency
t23a45ef9 = Độ trễ

# Down
ta4995035 = Tải xuống

# Up
t42f35940 = Tải lên

# Loss
t69db303a = Mất gói

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Hãy nhập độ trễ, tốc độ hoặc tỉ lệ mất gói để làm chậm

# Latency, ms
tefe18ab9 = Độ trễ, ms

# Down, kbit/s
t08c4ae33 = Tải xuống, kbit/s

# Up, kbit/s
td55cfe9a = Tải lên, kbit/s

# Loss, %
tea4af439 = Mất gói, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Chỉ các host này, như *.example.com; để trống là tất cả

# Not kept for next time: {e}
tdce4d67c = Chưa lưu được cho lần sau: { $e }

# Slow network
td51ddfce = Mạng chậm

# Off: traffic goes as fast as it can
tf0f34db0 = Tắt: traffic đi nhanh hết mức có thể

# Custom
t9525b78e = Tuỳ chỉnh

# Only these hosts
tfe16992b = Chỉ các host này

# Apply
tca77bbdb = Áp dụng

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Áp dụng từ request kế tiếp. Độ trễ được cộng vào mỗi lượt đi về; tốc độ là một đường truyền mà mọi flow bị làm chậm dùng chung; phần bị mất sẽ chờ để gửi lại, như TCP vẫn làm. Các host được để nguyên không bị làm chậm.

# Header line {line} has no ":"
t891b076d = Dòng header { $line } không có ":"

# Header line {line} has no name
te3247b96 = Dòng header { $line } không có tên

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
       *[other] { $n } byte không phải chữ: chúng được gửi đi nguyên như khi đến.
    }

# Nothing is paused
t8cf68b4c = Không có gì đang tạm dừng

# "{status}" is not a status
t40a529af = "{ $status }" không phải là một status

# REQUEST
t1f3f6dd2 = REQUEST

# RESPONSE
tc85f77de = RESPONSE

# MESSAGE TO THE SERVER
t270a6209 = MESSAGE TỚI SERVER

# MESSAGE TO THE APP
t7d928b05 = MESSAGE TỚI ỨNG DỤNG

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Không có gì đang tạm dừng. Hãy thêm quy tắc Tạm dừng ở trang Quy tắc để dừng traffic ở đây.

# {kind} on {url}
t1204e859 = { $kind } trên { $url }

# for {method} {url}
t093a86f6 = cho { $method } { $url }

# The request, before the server gets it
tc840fed6 = Request, trước khi server nhận

# The response, before the app gets it
t6683d72a = Response, trước khi app nhận

# A socket's message, before the server gets it
tda2ebc5d = Message của socket, trước khi server nhận

# A socket's message, before the app gets it
tbce07e9d = Message của socket, trước khi ứng dụng nhận

# Message
tae0ed984 = Message

# Send as it came
tb593e730 = Gửi nguyên như khi đến

# Drop
t108524ac = Bỏ

# Paused
tde281feb = Tạm dừng

# Nothing is waiting.
tea613769 = Không có gì đang chờ.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
       *[other] { $n } đang chờ được gửi tiếp.
    }

# Send all as they came
tdce25640 = Gửi tất cả nguyên như khi đến

# The list was not saved: {e}
tfceba380 = Danh sách chưa được lưu: { $e }

# {count} service: {names} | {count} services: {names}
tc1b8764c = { $count } service: { $names }

# https://api.example.com or http://localhost:50051
t14f9744c = https://api.example.com hoặc http://localhost:50051

# Reading…
t6aad8485 = Đang đọc…

# {count} file | {count} files
tde674bd8 = { $count } file

# {count} message | {count} messages
t8a1cd2f6 = { $count } message

# {count} service | {count} services
t51f71902 = { $count } service

# and {count} more problem | and {count} more problems
t591ce5ea = và { $count } lỗi khác

# Protobuf schemas
ta4401d1a = Schema Protobuf

# Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
te5f4f07c = Thư mục chứa file .proto, hoặc descriptor set tạo bằng protoc -o, để hiện body Protobuf và gRPC với tên field. Lời gọi gRPC tìm message theo path; Protobuf khác theo messageType trong content type. Chúng được đọc lại khi thay đổi.

# Add a folder…
tb5930e43 = Thêm thư mục…

# Add files…
ted08dbe5 = Thêm file…

# Read again
t2f0dcc5d = Đọc lại

# Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.
tf6948a5a = Hoặc hỏi một server có gRPC reflection để lấy file của nó. https:// đi qua tracewing; http:// là gRPC không TLS, nối thẳng tới server.

# Asking…
tf9cf22fe = Đang hỏi…

# Fetch
t533a3183 = Lấy về

# Find a message
te6f78626 = Tìm message

# Read as {message}
t87b6a380 = Đọc như { $message }

# Read as…
t1b90af95 = Đọc như…

# Requests to {path} on {host} are read as the message chosen.
t8826d766 = Request tới { $path } trên { $host } được đọc như message đã chọn.

# Answers from {path} on {host} are read as the message chosen.
tbbe012da = Response từ { $path } trên { $host } được đọc như message đã chọn.

# Automatic (gRPC path or messageType)
t134c9a06 = Tự động (path gRPC hoặc messageType)

# and {count} more: type to narrow them | and {count} more: type to narrow them
t70ff68f9 = và { $count } message khác: gõ để thu hẹp

# Read as
t8c0f1d8b = Đọc như

# Copy reading
td38c5cc8 = Copy phần đã đọc

# Nothing to read in this
t92a8694c = Không đọc được gì ở đây

# Write the server's address first
t4f68f91a = Hãy nhập địa chỉ server trước

# "{address}" is not a server's address
t8c4d03a6 = "{ $address }" không phải địa chỉ server

# The server has no services to describe
tfa5b7ad8 = Server không có service nào để mô tả

# The server sent no files for its services
tdca19bdc = Server không gửi file nào cho các service của nó

# The server answered {status}: it may not offer gRPC reflection
t345fdb40 = Server trả lời { $status }: có thể nó không có gRPC reflection

# The server answered {code}: it may not offer gRPC reflection. {message}
tb629329e = Server trả lời { $code }: có thể nó không có gRPC reflection. { $message }

# The server answered nothing: it may not offer gRPC reflection
t6bbb6207 = Server không trả lời gì: có thể nó không có gRPC reflection

# The password is kept in the system's keychain.
t3a48373c = Mật khẩu được giữ trong kho khoá của hệ thống.

# The password is kept in relay.toml, which only you can read.
tf84cb18b = Mật khẩu được giữ trong relay.toml, chỉ bạn đọc được.

# The password is kept in relay.toml, which only you can read: the keychain cannot be used ({why}).
t5a08b6d0 = Mật khẩu được giữ trong relay.toml, chỉ bạn đọc được: không dùng được kho khoá ({ $why }).

# New connections go through the {kind} proxy {address}.
tb5f1ef96 = Các kết nối mới đi qua proxy { $kind } { $address }.

# Connections go straight to servers.
t4a64a08f = Kết nối đi thẳng tới server.

# relay.toml cannot be used, so connections go straight: {problem}
tbc6a2da3 = Không dùng được relay.toml, nên kết nối đi thẳng: { $problem }

# http://proxy.example.com:8080 or socks5://proxy.example.com:1080
tb945a053 = http://proxy.example.com:8080 hoặc socks5://proxy.example.com:1080

# Go through another proxy
tc67dfdb1 = Đi qua một proxy khác

# Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080; the password is then kept in the system's keychain, not shown again. Empty, tracewing goes straight to servers.
t5c30a22f = Khi mạng công ty chỉ cho ra ngoài qua proxy của công ty, tracewing đi tiếp tới server qua proxy đó. Các app vẫn gửi traffic tới tracewing. Tên đăng nhập và mật khẩu ghi trong địa chỉ, như http://tên:mật-khẩu@proxy:8080; mật khẩu sau đó được giữ trong kho khoá của hệ thống và không hiện lại. Để trống thì tracewing đi thẳng tới server.

# Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host.
tcbc7d298 = Các host đi thẳng, theo quy ước NO_PROXY: example.com là chính nó và mọi host bên dưới, * là mọi host.

# Sent by the server, not asked for
td717c770 = Server tự gửi, không phải trả lời yêu cầu nào

# any
t2c29f04d = bất kỳ

# Failed
t5579c3d4 = Thất bại

# The server answered 200: a call that fails says so in the body.
ta74e5f73 = Server trả về 200: lời gọi thất bại nói điều đó trong body.

# {count} item | {count} items
t534e422e = { $count } mục

# {count} field | {count} fields
t0039810a = { $count } trường

# and {count} more
tbdb1ff6a = và { $count } nữa

# {count} byte that does not read as one | {count} bytes that do not read as one
ta7ee514c = { $count } byte không đọc được thành một transaction

# {program}, {count} bytes
t1ce6ad3b = { $program }, { $count } byte

# succeeded
t6b166ff2 = thành công

# failed: it was mined, and what it asked for did not happen
t46ea918c = thất bại: đã vào block, nhưng việc nó yêu cầu đã không xảy ra

# none: it makes a contract
t188b58f6 = không có: nó tạo contract

# {count} byte, which the signature does not say how to read | {count} bytes, which the signature does not say how to read
t36ffce30 = { $count } byte mà chữ ký không cho biết cách đọc

# {count} place | {count} places
t0965779e = { $count } vị trí

# {count} blob | {count} blobs
t95369f92 = { $count } blob

# 0, the kind from before EIP-2718
tc2fde48e = 0, loại có từ trước EIP-2718

# The bodies were not kept, so what the call sent cannot be shown.
teaf455ca = Body không được lưu, nên không hiện được lời gọi đã gửi gì.

# and {count} more calls
tf0ac7103 = và { $count } lời gọi nữa

# {count} function | {count} functions
t9ffc3c7c = { $count } hàm

# {count} event | {count} events
t4dd03992 = { $count } sự kiện

# {count} error | {count} errors
t4fba4c4e = { $count } lỗi

# Contract ABIs
t08fb00f2 = ABI của contract

# Folders or files of ABI JSON, as solc, Hardhat and Foundry write it, to name the functions, events and errors in a JSON-RPC call instead of showing four bytes. They are read again when they change. Nothing is sent anywhere: a signature is worked out from the ABI on this machine.
tfadffbd2 = Thư mục hoặc file ABI JSON, dạng solc, Hardhat và Foundry sinh ra, để gọi tên hàm, sự kiện và lỗi trong một lời gọi JSON-RPC thay vì hiện bốn byte. Chúng được đọc lại khi thay đổi. Không gửi gì ra ngoài: chữ ký được tính từ ABI trên máy này.

# Block
tbd074002 = Chặn

# Answer from a file
td8edab50 = Trả lời bằng một file

# Send to another host
te3cd85d7 = Gửi tới host khác

# Set a request header
tfad4aaaa = Đặt header cho request

# Set a response header
tcbbc20d0 = Đặt header cho response

# Remove a request header
t5228dd84 = Xóa header của request

# Remove a response header
teeb5735a = Xóa header của response

# Set a query parameter
t73d9419b = Đặt tham số query

# Remove a query parameter
t12633da9 = Xóa tham số query

# Change the request body
t41101201 = Đổi body của request

# Change the response body
tdd259b73 = Đổi body của response

# Change the status
t4bbdfba8 = Đổi status

# Pause the request
t40ecc39b = Tạm dừng request

# Pause the response
t18a34799 = Tạm dừng response

# Pause a socket's messages to the server
te5d065fe = Tạm dừng message của socket gửi tới server

# Pause a socket's messages to the app
t482173e8 = Tạm dừng message của socket gửi tới ứng dụng

# Run a script
t65a52970 = Chạy một script

# Status to answer with, like 403
t8f3ebb6c = Status trả về, như 403

# Full path of a file or folder
t9fb9ae4c = Đường dẫn đầy đủ của một file hoặc thư mục

# Full path of a .js file with onRequest or onResponse
t01bacb53 = Đường dẫn đầy đủ tới file .js có onRequest hoặc onResponse

# Where to, like http://localhost:3000
tdfedd004 = Gửi tới đâu, như http://localhost:3000

# Header's name
t80793085 = Tên header

# Its value
t43968778 = Giá trị của nó

# Parameter's name
t313c8e35 = Tên tham số

# Text to find, or empty for the whole body
t8255fc87 = Chữ cần tìm, hoặc để trống để thay cả body

# What to put in its place
t03671fd5 = Nội dung thay vào

# Status the client gets, like 500
t5e4d3088 = Status client nhận được, như 500

# "{first}" is not a status
td43128d3 = "{ $first }" không phải là một status

# Block with {status}
t1727e984 = Chặn với { $status }

# Answer from {path}
taefc08fa = Trả lời bằng { $path }

# Send to {to}
t5e256873 = Gửi tới { $to }

# Request: {header}: {value}
taa230766 = Request: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Response: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Request: không có { $header }

# Response: no {header}
t23960a2e = Response: không có { $header }

# Query: {name}={value}
t0ee6c1c0 = Query: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Query: không có { $name }

# Status {status}
tad048e53 = Status { $status }

# Run {path}
t2f9fde3f = Chạy { $path }

# Request body: {replace}
te4856c36 = Body của request: { $replace }

# Response body: {replace}
t8600dc42 = Body của response: { $replace }

# Request body: {find} → {replace}
taec471a9 = Body của request: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Body của response: { $find } → { $replace }

# Name
t0fe07306 = Tên

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, như api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Method, hoặc để trống cho tất cả

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
       *[other] Đã ghi { $n } quy tắc vào { $path }
    }

# The rules were not written: {e}
te7ac257f = Chưa ghi được các quy tắc: { $e }

# Import
t573e0af4 = Nhập

# {path} could not be read: {e}
t32a3ce94 = Không đọc được { $path }: { $e }

# Nothing was imported: {e}
tb9f7257a = Không nhập được gì: { $e }

# Failed: {why}
tfce667e2 = Lỗi: { $why }

# Ran once. Last said: {last} | Ran {n} times. Last said: {last}
t3e82a85a = Đã chạy { $n } lần. Dòng cuối: { $last }

# Ran once. | Ran {n} times.
t47b9c718 = Đã chạy { $n } lần.

# Edit
tc2c76cb1 = Sửa

# Delete
t5797ea6a = Xoá

# Change this rule
t9160f393 = Sửa quy tắc này

# A new rule
t55a722a0 = Quy tắc mới

# It waits under Paused until you send it on, changed or not.
tca566a57 = Nó chờ ở trang Tạm dừng cho tới khi bạn gửi tiếp, có sửa hay không.

# Save
t4d2d5d68 = Lưu

# Cancel
t35afca3b = Huỷ

# Rules
t428b2d80 = Quy tắc

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Rule có hiệu lực từ request kế tiếp, kể cả khi đang tạm dừng ghi. Từ trên xuống: rule chặn, trả file hoặc đổi host đầu tiên khớp sẽ thắng, mọi rule khác khớp đều áp dụng, theo thứ tự. Rule đổi body hay status thì đổi những gì server gửi về.

# Import rules…
t5edc205d = Nhập quy tắc…

# Export rules…
t6e09ca20 = Xuất quy tắc…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
       *[other] Đã thêm { $n } quy tắc
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
       *[other] { $n } quy tắc đã có sẵn
    }

# left out: {why}
ta81e95e2 = bỏ qua: { $why }

# Copy all
t6aef5c37 = Sao chép tất cả

# Too large to copy all
t2fe8ff9f = Quá lớn để sao chép tất cả

# No flows
t6dc463eb = Không có flow

# Name it to keep it
t52c1a402 = Đặt tên để giữ lại

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · do phiên bản khác ghi, nên không mở được

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
       *[other] { $count } flow · { $size }
    }

# Recording now
t3cb3e6fc = Đang ghi

# Export HAR
t4d6458cc = Xuất HAR

# Delete for good?
ta7737f13 = Xoá vĩnh viễn?

# Sessions
tb69a8aec = Phiên

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Phiên có tên được giữ cho tới khi bạn xoá. Trong các phiên còn lại, { $kept } phiên mới nhất được giữ.

# Working…
td5507854 = Đang xử lý…

# Open a session file
tae8622fc = Mở file phiên

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, .saz của Fiddler, hoặc .chlsj của Charles

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = File HAR export ra bỏ đi cookie, token, key và mật khẩu, trong header, URL và body

# Other devices
t9e7d8362 = Thiết bị khác

# Let other devices use the proxy
t13e626b3 = Cho thiết bị khác dùng proxy

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = Điện thoại và máy tính khác trong mạng của bạn có thể gửi traffic qua đây. Bất kỳ ai khác trong mạng cũng vậy: hãy tắt khi ở mạng không tin cậy.

# Servers
tcd0cf113 = Server

# Trust what this computer trusts
tfea2d91b = Tin những gì máy này tin

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = Server có chứng chỉ từ một tổ chức cấp chứng chỉ được cài trên máy này, chẳng hạn CA của công ty, sẽ được kết nối như trình duyệt trên máy này vẫn làm. Khi tắt, chỉ các tổ chức công khai được tin. Có hiệu lực khi tracewing khởi động lại.

# AI tools
t294d7ea4 = Công cụ AI

# Let AI tools see tokens and cookies
te1d26d3c = Cho công cụ AI thấy token và cookie

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = Công cụ AI đọc traffic đã ghi qua MCP server của tracewing. Khi tắt, giá trị của Authorization, Cookie và Set-Cookie không được đưa cho chúng. Áp dụng ngay.

# Let AI tools add rules and send requests
t663de1ba = Cho công cụ AI thêm rule và gửi request

# Through the MCP server, an AI tool may then add and remove rules, which change traffic as yours do, and send requests through tracewing, again or changed, which reach their servers. Off, it can only read. Applies at once.
t46d53486 = Qua MCP server, công cụ AI khi đó có thể thêm và xoá rule, vốn thay đổi traffic như rule của bạn, và gửi request qua tracewing, gửi lại hoặc đã sửa, tới tận server thật. Tắt thì nó chỉ đọc được. Áp dụng ngay.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Giữ app không dùng HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = HTTP/3 chạy trên UDP, thứ không proxy nào chuyển được, nên app dùng nó sẽ đi vòng qua tracewing. Tuỳ chọn này bỏ lời mời dùng HTTP/3 khỏi response của server, để app ở lại HTTP/2. Flow vẫn hiện lời mời đúng như server gửi.

# Size of a session
ta20fa102 = Kích thước một phiên

# Past this, the oldest flows are dropped.
td2914f46 = Vượt quá mức này, các flow cũ nhất bị xoá.

# Room to leave on the disk
t8674a3b3 = Dung lượng chừa lại trên đĩa

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Khi còn trống ít hơn mức này, flow vẫn được liệt kê nhưng body không được giữ.

# Earlier sessions to keep
td356194a = Số phiên cũ được giữ

# Older ones are deleted when a new session starts.
t66556513 = Phiên cũ hơn bị xoá khi một phiên mới bắt đầu.

# Bodies
tdb4d0409 = Body

# Most kept of one body
t777508dd = Phần lớn nhất được giữ của một body

# It still reaches the client whole.
t386fee28 = Client vẫn nhận đủ toàn bộ.

# Most unpacked for viewing
tc3fe2efd = Phần lớn nhất được giải nén để xem

# Stops a small download that unpacks to many GB.
tf9c49515 = Chặn một file tải nhỏ mà giải nén ra hàng GB.

# Small body size
ta8996af1 = Ngưỡng body nhỏ

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Body nhỏ hơn chờ trong bộ nhớ và ghi chung một file. Body lớn hơn có file riêng ngay khi tới.

# Compress plain text on disk
tee503c2e = Nén văn bản thường trên đĩa

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Tiết kiệm khoảng một nửa dung lượng, và được bỏ qua khi traffic quá nhanh.

# Compression level
td386c9ef = Mức nén

# 1 to 19. Higher is smaller and slower.
td42a987d = Từ 1 tới 19. Càng cao càng nhỏ và càng chậm.

# Search
tc646a2c9 = Tìm kiếm

# Search URLs
t0c3e0b3d = Tìm trong URL

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Tìm bất kỳ phần nào của URL khi bạn gõ. Tốn rất ít.

# Search bodies
t8eec0293 = Tìm trong body

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Tìm chữ trong body. Có thể tốn dung lượng đĩa ngang với chính các body.

# Searched part of each body
tab293b21 = Phần được tìm của mỗi body

# Words past this are not found. More takes more disk.
t0bf65f4a = Chữ nằm sau mức này sẽ không tìm thấy. Càng lớn càng tốn đĩa.

# Under heavy traffic
ta99888cb = Khi traffic nhiều

# Memory for bodies waiting to be written
t7d6ff799 = Bộ nhớ cho body đang chờ ghi

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Vượt quá mức này, flow được liệt kê mà không có body. Traffic không bao giờ bị giữ lại.

# Memory for large bodies waiting to be written
t07b9de08 = Bộ nhớ cho body lớn đang chờ ghi

# Past this, large bodies stop being kept.
tfe61a4f3 = Vượt quá mức này, body lớn không còn được giữ.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }" không phải là số nguyên

# {key}: must not be negative
t9c4debaa = { $key }: không được là số âm

# After a restart
t58dcea2d = Sau khi khởi động lại

# Every setting is back to its default.
t763785e7 = Mọi cài đặt đã về mặc định.

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d = { $n } thay đổi sẽ có hiệu lực khi tracewing khởi động lại.

# Settings
t4b058728 = Cài đặt

# Undo
t71fd4acf = Hoàn tác

# Back to defaults
ta3162a13 = Trở về mặc định

# Hosts not to decrypt
tb457f0b3 = Host không giải mã

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = Mỗi dòng một mục: một host, hoặc *.example.com cho host đó và mọi host bên dưới. Traffic của chúng đi qua mà không bị đọc. Danh sách này áp dụng ngay.

# Authorities trusted for servers
t0a308a8a = CA được tin cho server

# tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in {folder}. They are read when tracewing starts.
t571051f9 = tracewing tin các server được các CA công khai xác nhận, và các CA mà máy này tin. Nó cũng tin các CA sau: CA riêng của công ty, hoặc CA của chính bạn, đặt dưới dạng file .pem trong { $folder }. Chúng được đọc khi tracewing khởi động.

# Left out: {why}
t4a567d82 = Bỏ qua: { $why }

# The folder has changed since tracewing started: restart it to use what it holds now.
t65f2d800 = Thư mục đã thay đổi từ khi tracewing khởi động: hãy khởi động lại để dùng nội dung mới.

# Open the folder
tc7d7e212 = Mở thư mục

# Read it again
td72efd5e = Đọc lại

# TLS keys for Wireshark
tcb4394af = Khoá TLS cho Wireshark

# Writes the secrets of each HTTPS connection, from apps to tracewing and from it to servers, to a file Wireshark reads: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Anyone with the file can read the traffic it covers. Off again when tracewing starts, unless SSLKEYLOGFILE names a file.
t2f92ac10 = Ghi khoá bí mật của mỗi kết nối HTTPS, từ ứng dụng tới tracewing và từ tracewing tới server, vào một file mà Wireshark đọc được: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Ai có file này đều đọc được lưu lượng mà nó bao gồm. Tự tắt khi tracewing khởi động lại, trừ khi SSLKEYLOGFILE chỉ định một file.

# Cannot write to it: {why}
t956ecc4b = Không ghi được vào đó: { $why }

# Copy the path
t095d064e = Sao chép đường dẫn

# Language
t9a73db9b = Ngôn ngữ

# System ({system_name})
tb5c7a1d4 = Theo hệ thống ({ $system_name })

# Applies at once.
tefb7a0f9 = Áp dụng ngay.

# Your browser
tb5a48dd2 = Trình duyệt của bạn

# Its private key is in the system keychain.
t62a47808 = Khoá riêng của nó nằm trong keychain của hệ thống.

# Its private key is in ca.key, which only you can read.
td9c6165d = Khoá riêng của nó nằm trong ca.key, chỉ bạn đọc được.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = Khoá riêng của nó nằm trong ca.key, chỉ bạn đọc được, vì keychain của hệ thống không nhận: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Mở trang này. Nếu trình duyệt cảnh báo về chứng chỉ, hoặc không tìm thấy trang, thì một trong các bước trên chưa xong với trình duyệt đó.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } đi qua tracewing và tin chứng chỉ của nó.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } đi qua tracewing, nhưng đã mở trang không qua HTTPS. Hãy mở lại bằng https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = Điện thoại hoặc máy tính khác cũng có thể gửi traffic qua đây. Hãy bật “Cho thiết bị khác dùng proxy” trong Cài đặt.

# this computer's address, port {port}
t2188b3f4 = địa chỉ của máy này, cổng { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = Trên thiết bị, đặt proxy Wi-Fi là { $addr }. Rồi mở http://{ $host } trên đó, tải chứng chỉ và tin nó. Flow của thiết bị hiện địa chỉ của nó, và lọc được bằng device:. Đặt tên cho thiết bị từ một flow của nó, ở tab Kết nối.

# Looking for it…
t50ee7452 = Đang tìm…

# Installing. The system may ask you to confirm.
ta41c38ec = Đang cài. Hệ thống có thể hỏi bạn xác nhận.

# Removing. The system may ask you to confirm.
t62b22726 = Đang gỡ. Hệ thống có thể hỏi bạn xác nhận.

# This computer trusts it.
t27d6bacc = Máy này tin chứng chỉ đó.

# Installed in: {places}.
tb5bcc244 = Đã cài vào: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Các công cụ dòng lệnh tin chứng chỉ, nhưng trình duyệt có kho riêng. Cho tới khi nó có trong đó, HTTPS qua tracewing sẽ lỗi trong trình duyệt.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing đọc HTTPS bằng một chứng chỉ tạo riêng cho máy này. Cho tới khi chứng chỉ được tin, HTTPS qua tracewing sẽ lỗi.

# Trust the certificate
t26618699 = Tin chứng chỉ

# Install
tb50cbd40 = Cài

# Also for command-line tools
t0db8faea = Cả cho công cụ dòng lệnh

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Gỡ chứng chỉ khỏi mọi kho trên máy này? Sau đó HTTPS qua tracewing sẽ lỗi cho tới khi cài lại. tracewing vẫn giữ chứng chỉ để cài lại sau.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = Trên Linux, phải có certutil thì mới bảo được trình duyệt tin chứng chỉ. Hãy cài nó, rồi bấm Cài ở đây:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = Trên Linux, phải có certutil thì mới bảo được trình duyệt tin chứng chỉ. Hãy cài gói NSS tools của bản phân phối, rồi bấm Cài ở đây.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = Desktop này không có thiết lập system proxy nào mà tracewing đổi được. Hãy mở riêng một trình duyệt trong mục Trình duyệt, hoặc mở terminal, ở bên dưới.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } không có thiết lập system proxy nào mà tracewing đổi được. Hãy mở riêng một trình duyệt trong mục Trình duyệt, hoặc mở terminal, ở bên dưới.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Các app dùng thiết lập của hệ thống giờ đi qua { $addr }. Thiết lập trở lại như cũ khi tracewing đóng, hoặc khi nó bị crash.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Đưa traffic của máy này qua tracewing, với các app dùng thiết lập của hệ thống. Hoặc tự trỏ một app tới { $addr }.

# Turn on the system proxy
t52b88441 = Bật system proxy

# Turn off
tcd03e04b = Tắt

# Turn on
t65a0c5a7 = Bật

# Check it in your browser
tbed6e83a = Kiểm tra trong trình duyệt

# Open the test page
te4e36f65 = Mở trang kiểm tra

# Set up
t43c06abc = Thiết lập

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Ba bước để xem traffic HTTPS của máy này.

# See traffic
t980eee79 = Xem traffic

# the browsers' own stores
tddf6836c = kho riêng của các trình duyệt

# Trusts the certificate.
tc5bba858 = Tin chứng chỉ.

# Does not trust the certificate yet.
tf2a0b8c5 = Chưa tin chứng chỉ.

# Looking…
t284579ba = Đang tìm…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Chưa có profile. Hãy mở nó một lần, rồi tìm lại.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 = { $holding } trong { $n } profile của nó tin chứng chỉ.

# Has not opened the test page yet.
t67f9cb49 = Chưa mở trang kiểm tra.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Đã mở trang kiểm tra qua tracewing, bằng HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Đã mở trang kiểm tra qua tracewing, nhưng không bằng HTTPS.

# Browsers
td580efd4 = Trình duyệt

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Mỗi trình duyệt có cách riêng để quyết định tin gì và gửi traffic đi đâu. Kiểm tra mở trang kiểm tra trong trình duyệt đó. Mở riêng chạy nó với một profile riêng đi qua tracewing, không đụng tới system proxy.

# Look again
t98416222 = Tìm lại

# No browser found on this computer.
t0e46bb34 = Không tìm thấy trình duyệt nào trên máy này.

# Check
t592c8de7 = Kiểm tra

# Open on its own
t0da7783c = Mở riêng

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = Cổng UDP 443 đang bị chặn trên máy này, nên app chuyển từ HTTP/3 về HTTP/2 và đi qua tracewing. Việc chặn dừng khi tracewing đóng, hoặc khi nó bị crash.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Một số app dùng HTTP/3, chạy trên UDP, và đi vòng qua proxy. Chặn cổng UDP 443 trên máy này khiến chúng quay về HTTP/2 qua tracewing. Cần mật khẩu quản trị, và việc chặn dừng khi tracewing đóng. Trong lúc đó, VPN dùng cổng UDP 443 sẽ không hoạt động.

# Apps on HTTP/3
t2cfe5270 = App dùng HTTP/3

# Stop blocking
t318358d8 = Ngừng chặn

# Block HTTP/3
t88231e40 = Chặn HTTP/3

# Or one app at a time
t91c63f1c = Hoặc từng app một

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Một terminal đi qua tracewing, còn system proxy và mọi app khác vẫn giữ nguyên. Với trình duyệt, nút Mở riêng trong mục Trình duyệt làm điều tương tự với một profile riêng.

# Open a terminal
t4b629c72 = Mở terminal

# iPhone and iPad
ta550cc97 = iPhone và iPad

# Flutter apps
ta04ee397 = App Flutter

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. Trong Cài đặt, Wi-Fi, chọn mạng, rồi Định cấu hình proxy, Thủ công: máy chủ { $server }, cổng { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Quét mã bằng camera, hoặc mở { $page } trong Safari, rồi tải chứng chỉ. Cho phép tải về.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. Trong Cài đặt, Cài đặt chung, VPN & Quản lý thiết bị, chọn profile tracewing CA và cài đặt.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. Trong Cài đặt, Cài đặt chung, Giới thiệu, Cài đặt tin cậy chứng nhận, bật tin cậy hoàn toàn cho tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Mở https://{ $host } trong Safari. Nếu trang báo đã chạy, các app dùng thiết lập của hệ thống sẽ đi qua tracewing.

# When you are done
t62466f08 = Khi đã xong

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. Vào Settings, Wi-Fi, chọn mạng, rồi Configure Proxy: Off. Nếu để nguyên, thiết bị sẽ mất Internet trên mạng này khi tracewing tắt.

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. Vào Settings, General, VPN & Device Management, chọn tracewing CA và gỡ profile. Quyền tin cậy đầy đủ của nó cũng mất theo.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. Trong Cài đặt, Wi-Fi, sửa mạng, và ở phần Nâng cao đặt proxy là Thủ công: máy chủ { $host }, cổng { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Quét mã, hoặc mở { $page } trong trình duyệt, rồi tải chứng chỉ.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. Trong Cài đặt, Bảo mật, Mã hoá và thông tin xác thực (tên gọi khác nhau tuỳ hãng), chọn Cài đặt chứng chỉ, rồi Chứng chỉ CA, và chọn tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Từ Android 7, app chỉ tin chứng chỉ do người dùng thêm nếu app cho phép. Với bản debug của app của bạn, hãy thêm res/xml/network_security_config.xml:

# and point to it from AndroidManifest.xml:
t02ad844b = và trỏ tới nó trong AndroidManifest.xml:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Các app khác vẫn từ chối chứng chỉ: traffic của chúng hiện thành flow lỗi.

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. Vào Settings, Wi-Fi, sửa mạng, và đặt proxy về None. Nếu để nguyên, thiết bị sẽ mất Internet trên mạng này khi tracewing tắt.

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. Vào Settings, Security, Encryption & credentials, User credentials, chọn tracewing và gỡ nó. Xóa luôn tracewing.crt trong Downloads.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = HttpClient của Dart, mà http và Dio của Flutter dùng, không theo thiết lập proxy của điện thoại, và trên Android không tin chứng chỉ do người dùng thêm. Trong bản debug, hãy gửi nó qua đây:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback cho mọi chứng chỉ đi qua, vì vậy chỉ dùng cho bản debug.

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = Bỏ TracewingProxy và dòng của nó trong main() trước khi phát hành, dù chúng không làm gì ngoài bản debug. Điện thoại không bị thay đổi gì.

# Open settings
t1336b4d6 = Mở cài đặt

# Phones and other devices
t2e3c591c = Điện thoại và thiết bị khác

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Điện thoại, máy tính bảng hoặc máy tính khác trong mạng này có thể đi qua tracewing tại { $proxy }. Mã QR mở trang có chứng chỉ. Flow của thiết bị hiện địa chỉ của nó, lọc được bằng device:, và đặt tên được từ một flow của nó, ở tab Kết nối.

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = Thiết bị khác không kết nối được tới tracewing tại một địa chỉ của máy này: { $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f = { $n } thiết bị đã đi qua tracewing ({ $names }). Khi đã xong, hãy tắt proxy Wi-Fi trên đó như hướng dẫn bên dưới: nếu để nguyên, thiết bị sẽ mất Internet trên mạng này khi tracewing tắt.

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Đã thiết lập. Chứng chỉ nằm trong kho của hệ thống cho tới khi thiết bị khởi động lại.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Đã thiết lập. Chứng chỉ nằm trong Downloads: trong Cài đặt vừa mở, chọn Mã hoá và thông tin xác thực, Cài đặt chứng chỉ, Chứng chỉ CA, rồi chọn tracewing.crt.

# Apps that trust only the system's certificates will still refuse it. On a device adb can root, as an emulator with a Google APIs image, tracewing puts it among the system's instead.
tfd0c5617 = Các ứng dụng chỉ tin chứng chỉ hệ thống vẫn sẽ từ chối nó. Trên thiết bị mà adb root được, như emulator dùng image Google APIs, tracewing sẽ đặt nó vào kho hệ thống.

# An emulator.
t14b6b86d = Một emulator.

# On USB.
t67dafc14 = Qua USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Hãy chấp nhận lời hỏi gỡ lỗi USB trên thiết bị, rồi tìm lại.

# Still starting. Look again in a moment.
t13f68b64 = Vẫn đang khởi động. Lát nữa hãy tìm lại.

# adb says it is {state}.
t2b7d71da = adb báo thiết bị đang { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Đặt proxy trên một Android emulator, hoặc điện thoại cắm USB có bật gỡ lỗi USB, và cài chứng chỉ. Traffic đi qua ADB, nên không cần chung mạng.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = Không tìm thấy adb. Hãy cài Android Studio, hoặc platform tools của Android, rồi tìm lại.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Chưa có thiết bị nào kết nối. Hãy mở một emulator, hoặc cắm điện thoại có bật gỡ lỗi USB, rồi tìm lại.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Thiết lập một thiết bị để gửi traffic của nó qua đây. Hoàn tác, hoặc đóng tracewing, sẽ gỡ proxy khỏi thiết bị.

# Android emulators and USB devices
t379e433c = Android emulator và thiết bị USB

# Find devices
t54d4af39 = Tìm thiết bị

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = System proxy và việc chặn HTTP/3 tự trả về khi tracewing tắt hoặc bị crash, thiết bị Android thiết lập qua USB cũng vậy. Chứng chỉ vẫn được tin cho tới khi nút Remove ở bước 1 gỡ nó. Điện thoại thiết lập bằng tay vẫn giữ proxy Wi-Fi và chứng chỉ cho tới khi được gỡ trên chính nó: hướng dẫn của từng loại thiết bị nói cách làm. Thiết bị khác vẫn dùng được proxy của máy này cho tới khi tắt tùy chọn đó trong Settings.

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code và các công cụ AI khác có thể liệt kê và đọc các flow tracewing ghi, qua MCP server của nó. Token và cookie bị giữ lại trừ khi Cài đặt cho phép. Trong Claude Code:

# Elsewhere, in the tool's MCP settings:
tec38833c = Ở các công cụ khác, trong phần cài đặt MCP:

# Flows
t4db77db2 = Flow

# Choose the flow above or below
tecb3adec = Chọn flow ở trên hoặc ở dưới

# The first or the last flow
t7e278a00 = Flow đầu tiên hoặc cuối cùng

# A page up or down
t23b1814b = Lên hoặc xuống một trang

# Filter
tf4a9c097 = Lọc

# Pause or resume recording
t6e21519a = Tạm dừng hoặc tiếp tục ghi

# Clear the list
tf2a55d1f = Xoá danh sách

# Compare the flow clicked with the chosen one
te42cd067 = So sánh flow được bấm với flow đang chọn

# The chosen flow
te0a729de = Flow đang chọn

# Its request
tb3970784 = Request của nó

# Its response
t221fc168 = Response của nó

# Send it again
tef7d1944 = Gửi lại

# Edit it and send it
t6bc99e5c = Sửa rồi gửi

# Copy it as cURL
tc99a11a3 = Sao chép dạng cURL

# The part before or after the one shown
tdf4221d9 = Tab trước hoặc sau tab đang xem

# Save the body shown
t20d9b7c0 = Lưu body đang xem

# Select all of the part shown
tf9168168 = Chọn tất cả trong phần đang xem

# Copy the selected text
t00b090d3 = Sao chép đoạn đã chọn

# Send it on, as edited
t8c35fa5a = Gửi tiếp, với phần đã sửa

# Send it on as it was
t16f7abb5 = Gửi tiếp như ban đầu

# Drop it
t4e7f48b9 = Bỏ nó

# The window
t2af18488 = Cửa sổ

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Thiết lập, Traffic, Quy tắc, Tạm dừng, Soạn request, Phiên, Cài đặt

# Show or hide the sidebar
t67adca04 = Hiện hoặc ẩn thanh bên

# Show or hide the inspector
ta5a7b9e9 = Hiện hoặc ẩn khung chi tiết

# The inspector beside the list or below it
t51e214f7 = Khung chi tiết bên cạnh hay bên dưới danh sách

# The next or the previous field
ta4b4bd5e = Ô kế tiếp hoặc ô trước

# These shortcuts
tbdf3b62b = Các phím tắt này

# Close them
tf3a4b1ef = Đóng bảng này

# Keyboard shortcuts
t31eaddb7 = Phím tắt

# Close
tcd86acc3 = Đóng

# The signature in this body now works out to {now}, not {was}: it is no longer the transaction that was signed. tracewing holds no keys, so it cannot sign the new one.
td0d9c66b = Chữ ký trong body này giờ tính ra { $now }, không phải { $was }: đây không còn là transaction đã được ký. tracewing không giữ khoá nên không ký lại được.

# What this body carries no longer reads as a signed transaction. tracewing holds no keys, so it cannot sign it again.
td97ae94b = Thứ body này mang không còn đọc được như một transaction đã ký. tracewing không giữ khoá nên không ký lại được.

# This body carries a signed transaction and its bytes have changed. The signatures are over the bytes as they were, and tracewing can neither check them nor make new ones.
ta6e69f9a = Body này mang một transaction đã ký và các byte của nó đã thay đổi. Chữ ký là cho các byte như lúc trước, và tracewing không kiểm được cũng không tạo mới được.

# The certificate: {e}
t0235648c = Chứng chỉ: { $e }

# The terminal
tf542b2f4 = Terminal

# The system proxy: {e}
tc99726ce = System proxy: { $e }

# Blocking HTTP/3: {e}
t024ec871 = Chặn HTTP/3: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx, hoặc chữ trong body

# A name, as iPhone test
t19f89214 = Một cái tên, như iPhone test

# A name, as Phone
tfce439bf = Tên, ví dụ Phone

# A message to send; {keys} sends it
tf9d9c09b = Message cần gửi; { $keys } để gửi

# The search index: {e}
te3c971c9 = Chỉ mục tìm kiếm: { $e }

# Click the flow to compare #{id} with.
tac1ec1d2 = Bấm vào flow để so sánh với #{ $id }.

# An earlier session: {name}
t73446a1d = Một phiên trước: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Chỉ đọc. Tìm kiếm bao quát thêm khi chỉ mục được hoàn tất. Việc ghi vẫn tiếp tục.

# Read-only. Recording goes on.
te2f35d9e = Chỉ đọc. Việc ghi vẫn tiếp tục.

# Back to live
t317c0453 = Về phiên đang ghi

# {shown} of {total}
tf2cc0563 = { $shown } trên { $total }

# Recording
te75cf20c = Đang ghi

# System proxy off · {addr}
t7e813d25 = System proxy tắt · { $addr }

# System proxy…
t29e7dcd0 = System proxy…

# System proxy on · {addr}
t319ad6b9 = System proxy bật · { $addr }

# Apps
t53d91aad = App

# Devices
t27acf320 = Thiết bị

# All hosts
tfc7bfe37 = Tất cả host

# No host has this in its name
t838c1d44 = Không có host nào chứa chuỗi này

# Hidden
t3c2dfe59 = Đã ẩn

# Hosts
tcf9aefb4 = Host

# Newest
t324b6d0b = Mới nhất

# No flow fits this filter
t7eb295f4 = Không có flow nào khớp bộ lọc này

# Nothing yet
tbd00747c = Chưa có gì

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Hãy bật system proxy ở trên, hoặc trỏ một app tới { $addr }. Để thử: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Bên cạnh danh sách

# Below the list
t23658daf = Bên dưới danh sách

# Drop it on a place · Esc puts it back
t0fa632ba = Thả vào một vị trí · Esc để đặt lại chỗ cũ

# Traffic
t4b589266 = Traffic

# Certificate trusted on this computer
t7b93f551 = Chứng chỉ được tin trên máy này

# Checking the certificate…
tbf8942c3 = Đang kiểm tra chứng chỉ…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Đang cài chứng chỉ; hệ thống có thể hỏi bạn xác nhận…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Đang gỡ chứng chỉ; hệ thống có thể hỏi bạn xác nhận…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS sẽ lỗi cho tới khi chứng chỉ được tin: thiết lập

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml không được dùng: { $problem }

# A setting was not saved: {problem}
tff1a5018 = Một cài đặt chưa được lưu: { $problem }

# The proxy stopped: {failure}
t04dcff11 = Proxy đã dừng: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = Đĩa gần đầy: body không được giữ

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
       *[other] { $count } flow không được ghi
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
       *[other] { $count } flow không có body
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
       *[other] { $count } flow cũ nhất bị xoá để không vượt quá { $size }
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
       *[other] { $count } flow
    }

# Export as HAR
t95b6b766 = Xuất HAR

# Shortcuts {keys}
t4ab0ecb4 = Phím tắt { $keys }

# Columns
tea156fdc = Cột

# base64url
tadfc8ac3 = base64url

# base64
tf04e6eba = base64

# Hex
t613f34ea = Hex

# Percent-encoded
t7c7bb5a9 = Mã hóa percent

# Escaped text
t195b3db9 = Văn bản có escape

# HTML entities
t54958ecb = Thực thể HTML

# Version
t5dcdd537 = Phiên bản

# Variant
t8433b026 = Biến thể

# UUID
t257b8832 = UUID

# ULID
tf19e9db9 = ULID

# ObjectId
t427abd77 = ObjectId

# Counter
tfc0f33c3 = Bộ đếm

# KSUID
te30e0b47 = KSUID

# Worker
tde5c1a77 = Worker

# Sequence
t64cec188 = Thứ tự

# Snowflake
te00a2b87 = Snowflake

# JSON, {n} fields
tc6a0fbd2 = JSON, { $n } trường

# JSON, {n} items
t06f5832d = JSON, { $n } phần tử

# JSON in a string
tc69f7572 = JSON trong một string

# Expires
tc7aa0523 = Hết hạn

# Issued
tb1835ecc = Cấp lúc

# Not before
td10dacd9 = Không trước

# Algorithm
t9945210a = Thuật toán

# State
t1deca496 = Trạng thái

# expired
t338eb632 = đã hết hạn

# JWT
t6076261e = JWT

# {n} row | {n} rows
td8d653d6 =
    { $n ->
       *[other] { $n } dòng
    }

# In hex and binary
tc31ef85f = Dạng hex và binary

# Bits set
tb67e8e09 = Các bit được set

# As a size
ta9175b27 = Dạng kích thước

# gzip
t1a451735 = gzip

# zlib
t7c5452c4 = zlib

# deflate
t1dfa0080 = deflate

# zstd
tcfa9c52a = zstd

# Brotli
t5b0660bd = Brotli

# in {amount}
tfda57c3e = sau { $amount }

# {amount} ago
t5f968014 = { $amount } trước

# Unix seconds
t6edf5796 = Unix giây

# Unix milliseconds
t0d28f4b7 = Unix milligiây

# Unix microseconds
tc705fe24 = Unix microgiây

# Unix nanoseconds
t235cb430 = Unix nanogiây

# ISO 8601
t402bdff3 = ISO 8601

# ISO 8601, no zone: read as UTC
t3bb6c7ca = ISO 8601, không có zone: đọc như UTC

# HTTP-date
te55df34c = HTTP-date

# Seconds
t7e12f48a = Giây

# Trace context
ta58c7c95 = Trace context

# Trace
t26f8e74e = Trace

# Span
taba837e1 = Span

# Sampled
t7e4a1d99 = Được sample

# yes
t4e9f3590 = có

# no
t62366ffa = không

# Parent span
t3cf3f9c3 = Span cha

# B3 trace
tb4f9b9bb = Trace B3

# Began
t3a1b8566 = Bắt đầu

# Amazon trace
t122e82a4 = Trace Amazon

# Server-Timing
tb79773df = Server-Timing

# Took
tcea1562e = Mất

# About
t58e096de = Mô tả

# Scheme
t9c9083a2 = Scheme

# Port
te2c53cc6 = Port

# User
te0a63e12 = Người dùng

# {n} fields
t98976d50 = { $n } trường

# Fragment
t7889e7fd = Fragment

# Query fields
t2f58f1e0 = Các trường query

# Field
t0fb83d07 = Trường

# Value
td147f96a = Giá trị

# Basic authentication
te161b66b = Xác thực Basic

# Password
t2cc30838 = Mật khẩu

# never kept
t83c57deb = không bao giờ lưu

# kept {span}
t01f67b47 = lưu { $span }

# kept {span} by shared caches
t5af054c4 = cache dùng chung lưu { $span }

# by the browser only
tfb7516b7 = chỉ bởi browser

# by anyone
t86fe63fe = bởi bất kỳ ai

# checked every time
t02076cba = kiểm tra mỗi lần

# not used once stale
t0f7f5bf9 = hết hạn là không dùng nữa

# never checked again
tc01144d0 = không kiểm tra lại

# Cache-Control
t3868d34d = Cache-Control

# Status code
t20b704bc = Mã trạng thái

# Cookie
tbdbabadf = Cookie

# data: URI, {media}
t514d8ddb = data: URI, { $media }
