# tracewing in Indonesian. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 = … { $count } baris { $closer }

# … {size} more
ta8dc6eab = … { $size } lagi

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc = { $size } { $format } · { $count } baris JSON

# The first {count} rows are shown.
t16aa1c6d = Hanya { $count } baris pertama yang ditampilkan.

# Unfold all
t3849d44e = Buka semua

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
       *[other] { $size } · { $count } baris
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · membaca { $percent }%

# Appearance: {now}. Click for {next}.
t332cfa6d = Tampilan: { $now }. Klik untuk { $next }.

# Method
t98a2c750 = Metode

# Status
t005ef20f = Status

# App
t81f51acc = Aplikasi

# Host
t302af18f = Host

# Path
teb66e456 = Path

# Operation
t218e02a6 = Operasi

# Size
ta6478e7c = Ukuran

# Time
tdfe4e404 = Waktu

# Number
t61e9eac0 = Nomor

# A body was not kept, so the two cannot be compared.
t9d6abb2a = Sebuah body tidak disimpan, jadi keduanya tidak bisa dibandingkan.

# {count} field differs | {count} fields differ
t6a8fff7f = { $count } field berbeda

# … {count} line the same | … {count} lines the same
tcc37e7ea = … { $count } baris sama

# Nothing to compare.
t9f32d942 = Tidak ada yang dibandingkan.

# URL
t7569633e = URL

# Protocol
t4a57ebd5 = Protokol

# Type
td155d06d = Tipe

# Sizes
td1a4a19d = Ukuran

# Body
t428a1095 = Body

# Headers
t334f5989 = Header

# Query
t9e89e783 = Query

# Request
t82454152 = Permintaan

# Response
t55a982fe = Respons

# Neither has headers kept.
tb43ef094 = Keduanya tidak menyimpan header.

# Trailers
t7c44fa89 = Trailer

# Neither URL has a query.
t4d8203e0 = Kedua URL tidak punya query.

# Comparing…
ta8565d31 = Membandingkan…

# No body
tfa4c2902 = Tanpa body

# {kind}, its first {size} compared
t1d894c39 = { $kind }, { $size } pertamanya dibandingkan

# Neither has a body.
tf2c47725 = Keduanya tidak punya body.

# The bodies are the same.
t85152e01 = Body-nya sama.

# {count} field differs. | {count} fields differ.
t0f651f77 = { $count } field berbeda.

# The first {count} are listed.
taae62a3e = Hanya { $count } pertama yang dicantumkan.

# {count} line differs. | {count} lines differ.
tf57a3157 = { $count } baris berbeda.

# Fields
t3308bd9c = Field

# Lines
t8a1a00dc = Baris

# Compare
td57a0a54 = Bandingkan

# #{a} beside #{b}: what only #{a} has is red, what only #{b} has is green.
t71ce2de9 = #{ $a } di samping #{ $b }: yang hanya ada di #{ $a } merah, yang hanya ada di #{ $b } hijau.

# Swap
te519d36e = Tukar

# Back to Traffic
t644042d2 = Kembali ke Traffic

# "{method}" is not a method
tacdcf5cb = "{ $method }" bukan metode

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = body permintaannya tidak disimpan utuh, jadi tidak bisa dikirim ulang

# name: value, one a line
t4df37bea = nama: nilai, satu per baris

# No flow is chosen
t550b0a98 = Belum ada flow yang dipilih

# This flow: {e}
t296762f8 = Flow ini: { $e }

# The body is not a {message}: {e}
t3220a10a = Body bukan { $message }: { $e }

# Protobuf {message}, written as JSON: it is sent as Protobuf.
t83b6cd6e = Protobuf { $message }, ditulis sebagai JSON: dikirim sebagai Protobuf.

# Stopped: no answer was read. What reached the server shows in Traffic.
t5a78825c = Dihentikan: tidak ada jawaban yang dibaca. Apa yang sampai ke server tampil di Traffic.

# Sending…
te946e7bf = Mengirim…

# Send
tf28e14cf = Kirim

# Stop
t4b7f7705 = Hentikan

# New
t8b242db1 = Baru

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … { $count } byte lagi: flow di Lalu Lintas memuat semuanya.

# Body · {message}
taa3cc3ff = Body · { $message }

# Compose
t18049f93 = Susun

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Permintaan buatan Anda sendiri, atau yang dikirim ulang. Permintaan ini lewat tracewing, jadi aturan berlaku dan permintaan juga muncul di Lalu Lintas.

# {version} both sides
t961a5ea0 = { $version } di kedua sisi

# {client} to the proxy, {server} to the server
t3d237646 = { $client } ke proxy, { $server } ke server

# Queued
t9e437c74 = Antre

# Connect
tdb89a899 = Koneksi

# Waiting
tc9450e48 = Menunggu

# Download
t6964bb19 = Unduhan

# No file picker: {e}
t6935fbd3 = Pemilih file tidak tersedia: { $e }

# Writing the HAR…
t725a0e89 = Menulis HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
       *[other] { $n } flow ditulis ke { $path }
    }

# The HAR was not written: {e}
tef8ec569 = HAR tidak ditulis: { $e }

# Open
t538b10e9 = Buka

# Reading {path}…
tc0e7c8f4 = Membaca { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
       *[other] , { $n } entri dilewati
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
       *[other] { $n } flow dibaca dari { $format }
    }

# The file was not read: {e}
t7f49510f = File tidak dibaca: { $e }

# it is no longer kept
ta7b4c380 = sudah tidak disimpan

# {size} saved to {path}
tc6932467 = { $size } disimpan ke { $path }

# The body was not saved: {e}
t189900b4 = Body tidak disimpan: { $e }

# Fiddler session
t9f5c89a3 = Sesi Fiddler

# Charles session
tb4f56f2b = Sesi Charles

# All
t75afe364 = Semua

# Errors
te9ed60a6 = Error

# Media
td10fcaa3 = Media

# Rule: {rule}
teca2074f = Aturan: { $rule }

# Events
t003420a4 = Peristiwa

# JSON lines
te8ce2db0 = Baris JSON

# Not decrypted
t8090a1ba = Tidak didekripsi

# open
td35ec4c9 = terbuka

# Reading the request…
t51c96267 = Membaca permintaan…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = Body permintaan tidak disimpan, jadi query-nya tidak bisa ditampilkan.

# An operation without a name
t38c4d30b = Operasi tanpa nama

# {n} of {count}: {name}
t53045460 = { $n } dari { $count }: { $name }

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Persisted query: server menyimpan teksnya, dan permintaan hanya mengirim hash-nya.

# Variables
t7996f062 = Variabel

# Cut
t58e25863 = Potong

# Copy
t658f3664 = Salin

# Paste
tb34a0288 = Tempel

# Select all
t6af21072 = Pilih semua

# This flow is gone.
t3343999e = Flow ini sudah tidak ada.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Host ini tidak didekripsi, jadi isi kirimannya tidak bisa dilihat.

# Still coming: it shows once it ends.
t42be95fb = Masih datang: tampil setelah selesai.

# No body.
t31e4d044 = Tanpa body.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Body ini tidak disimpan: disk hampir penuh, atau terlalu banyak yang menunggu disimpan.

# This body was not kept.
te46616b5 = Body ini tidak disimpan.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Body ini sudah tidak ada: terbuang bersama flow terlama, atau tidak bisa dibaca.

# Cannot open this body: {e}
t620a039b = Tidak bisa membuka body ini: { $e }

# Inspector
t4def410e = Inspektor

# Choose a flow to see it here
tc7be91e8 = Pilih flow untuk melihatnya di sini

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Tidak didekripsi. Koneksi baru ke host ini lewat tanpa dibaca.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Jika aplikasi gagal dengan host ini, mungkin aplikasi itu melakukan pinning sertifikat.

# Make a rule
t0ac0e062 = Buat aturan

# Decrypt this host
tb7367e2c = Dekripsi host ini

# Do not decrypt this host
t5fa113f6 = Jangan dekripsi host ini

# Request and response
t04e060e6 = Permintaan dan respons

# Request and response one at a time
t70397119 = Permintaan dan respons satu per satu

# Request and response side by side
t3d35ce69 = Permintaan dan respons berdampingan

# Request above the response
t83c63123 = Permintaan di atas respons

# Table
td70ceb7f = Tabel

# Pretty
t7e09470d = Rapi

# Raw
ta689aa91 = Mentah

# Picture · {size}
te97ed633 = Gambar · { $size }

# Messages {count}
te61b841a = Pesan { $count }

# Headers {headers}
t04b0146d = Header { $headers }

# Cookies {cookies}
td58eb84b = Cookie { $cookies }

# Query {query}
t4756f9f5 = Query { $query }

# Form {form}
tfa6e7203 = Form { $form }

# Connection
t09ea4ff9 = Koneksi

# Save body
te2c44424 = Simpan body

# Send again
tf5910557 = Kirim ulang

# Edit and send
tf837e542 = Edit dan kirim

# Copied
t8dc21305 = Disalin

# Copy as cURL
t866d1e3e = Salin sebagai cURL

# Click another flow…
t59253578 = Klik flow lain…

# Compare with…
t58dfe040 = Bandingkan dengan…

# Compare with the call before
tef15286c = Bandingkan dengan panggilan sebelumnya

# Copy as…
t9c1ad448 = Salin sebagai…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Gambar ini tidak bisa ditampilkan. Simpan body-nya untuk melihatnya.

# Opening…
tc40f2321 = Membuka…

# No response
t8c2c37b1 = Tidak ada respons

# Tunnel, not decrypted
tce693254 = Tunnel, tidak didekripsi

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# RPC: {method}
tf8162891 = RPC: { $method }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Lewat koneksi { $id }, yang dibuat sebelumnya: tidak ada waktu untuk menyambung.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Tidak ditemukan: mungkin sudah berakhir, atau milik pengguna lain

# On another device
t2b964a37 = Di perangkat lain

# CLIENT
t6d3cd99e = KLIEN

# Device
tbdcbb413 = Perangkat

# Program
tc4253b6b = Program

# Address
t700560f3 = Alamat

# SERVER
te3b27612 = SERVER

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = Info koneksi ini sudah tidak disimpan: ikut terbuang bersama flow yang lebih lama.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Tidak didekripsi: tracewing meneruskannya tanpa melihat isinya.

# No connection was made.
tf348eb92 = Tidak ada koneksi yang dibuat.

# None: it was answered without one.
tac71808e = Tidak ada: dijawab tanpa koneksi.

# {id}, given up during TLS
t0ec66a8c = { $id }, dibatalkan saat TLS

# {id}, made for this flow
t7fe32cfc = { $id }, dibuat untuk flow ini

# {id}, made before and used again
ta904c078 = { $id }, dibuat sebelumnya dan dipakai lagi

# Server
t6d7302f2 = Server

# None: plain HTTP
t05606349 = Tidak ada: HTTP biasa

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = SERVER TIDAK MENUNJUKKAN SERTIFIKAT

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = YANG DITUNJUKKAN SERVER, MILIKNYA SENDIRI DULU

# NO CERTIFICATES
te6f31dce = TIDAK ADA SERTIFIKAT

# CERTIFICATE
t1c902790 = SERTIFIKAT

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
       *[other] { $n } SERTIFIKAT, MILIK SERVER DULU
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing tidak melanjutkan dengan server ini: { $error }. Jika aplikasi memercayainya dengan cara lain, misalnya lewat CA perusahaan, pilih “Jangan dekripsi host ini” dan aplikasi akan memeriksa server itu sendiri.

# Cipher
t5b3bed6a = Cipher

# none
tada7afdb = tidak ada

# not valid until {when}
t0a14b2d8 = belum berlaku sampai { $when }

# expired {when}
t932538c2 = kedaluwarsa { $when }

# expires {when}
t679bcb51 = akan kedaluwarsa { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
       *[other] { $names }, dan { $n } lainnya
    }

# Issued by {issuer}
t15a1ce5a = Diterbitkan oleh { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Berlaku { $from } sampai { $to } · { $when }

# Names {names}
tda75e459 = Nama { $names }

# Serial {serial}
td1f0d98b = Serial { $serial }

# These headers were not kept.
tb8739fec = Header ini tidak disimpan.

# No headers.
tf3dacb5e = Tidak ada header.

# Trailers, sent after the body
t3447e9de = Trailer, dikirim setelah body

# File {filename} · {kind} · {size}
t6dbd239f = File { $filename } · { $kind } · { $size }

# no type
t902a6ece = tanpa tipe

# No cookies were sent.
t58e85ab2 = Tidak ada cookie yang dikirim.

# No cookies were set.
tf57dbabc = Tidak ada cookie yang diset.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · tanda tangan tidak diperiksa

# JWT · signature not checked
t05f8b533 = JWT · tanda tangan tidak diperiksa

# Header
t114f9980 = Header

# Payload
ta0639745 = Payload

# Expires
tc7aa0523 = Kedaluwarsa

# Issued
tb1835ecc = Diterbitkan

# Not before
td10dacd9 = Tidak sebelum

# in {amount}
tfda57c3e = dalam { $amount }

# {amount} ago
t5f968014 = { $amount } lalu

# Light
t8ad2f8cf = Terang

# Dark
t45570065 = Gelap

# System
t8f3184bc = Sistem

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
       *[other] di-stream dalam { $count } potongan
    }

# stopped: {stop}
tfa48fb30 = berhenti: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = token: { $input } masuk · { $output } keluar

# Reading the call…
t33b52a3c = Membaca panggilan…

# Answer
tf94ea0ff = Jawaban

# Thinking: {thinking}
t737ece33 = Berpikir: { $thinking }

# Calls {name}{id}
t017882e4 = Memanggil { $name }{ $id }

# Nothing yet.
td9b71d16 = Belum ada.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
       *[other] { $text }… dan { $count } karakter lagi
    }

# Copy the answer
t69975ed7 = Salin jawaban

# Asked
t4e3af9c3 = Ditanyakan

# Tools offered: {tools}
tb871bcf4 = Alat yang ditawarkan: { $tools }

# Not kept
tba30b5fa = Tidak disimpan

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Pesan ini tidak disimpan: terlalu banyak yang menunggu, atau disk hampir penuh.

# This message is empty.
tdb0ec01f = Pesan ini kosong.

# No messages yet
t51e7624c = Belum ada pesan

# {said} kept, of {size}
t3558745a = { $said } disimpan, dari { $size }

# The socket has closed.
t8d883a0f = Soket sudah tertutup.

# Too many are still waiting to go. Try again.
tbea55dfd = Terlalu banyak yang masih menunggu dikirim. Coba lagi.

# To the server
te1ef193a = Ke server

# To the app
t1bcabce4 = Ke aplikasi

# Layouts
t17809004 = Tata letak

# Keep the panels as they are now under a name, to come back to them.
t2b6aeb69 = Simpan susunan panel saat ini dengan sebuah nama, untuk kembali ke sana nanti.

# Keep
t555cbe68 = Simpan

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } bukan bilangan bulat

# Latency
t23a45ef9 = Latensi

# Down
ta4995035 = Unduh

# Up
t42f35940 = Unggah

# Loss
t69db303a = Loss

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Isi latensi, kecepatan, atau loss untuk memperlambatnya

# Latency, ms
tefe18ab9 = Latensi, ms

# Down, kbit/s
t08c4ae33 = Unduh, kbit/s

# Up, kbit/s
td55cfe9a = Unggah, kbit/s

# Loss, %
tea4af439 = Loss, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Hanya host ini, seperti *.example.com; semua jika kosong

# Not kept for next time: {e}
tdce4d67c = Tidak disimpan untuk lain kali: { $e }

# Slow network
td51ddfce = Jaringan lambat

# Off: traffic goes as fast as it can
tf0f34db0 = Mati: lalu lintas berjalan secepat mungkin

# Custom
t9525b78e = Kustom

# Only these hosts
tfe16992b = Hanya host ini

# Apply
tca77bbdb = Terapkan

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Berlaku mulai permintaan berikutnya. Latensi ditambahkan ke setiap round trip; kecepatan adalah satu link yang dipakai bersama oleh semua flow yang diperlambat; potongan yang hilang menunggu dikirim ulang, seperti pada TCP. Host lain tidak diperlambat.

# Header line {line} has no ":"
t891b076d = Baris header { $line } tidak punya ":"

# Header line {line} has no name
te3247b96 = Baris header { $line } tidak punya nama

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
       *[other] { $n } byte yang bukan teks: diteruskan apa adanya.
    }

# Nothing is paused
t8cf68b4c = Tidak ada yang dijeda

# "{status}" is not a status
t40a529af = "{ $status }" bukan status

# REQUEST
t1f3f6dd2 = PERMINTAAN

# RESPONSE
tc85f77de = RESPONS

# MESSAGE TO THE SERVER
t270a6209 = PESAN KE SERVER

# MESSAGE TO THE APP
t7d928b05 = PESAN KE APLIKASI

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Tidak ada yang dijeda. Tambahkan aturan Jeda di layar Aturan untuk menghentikan lalu lintas di sini.

# {kind} on {url}
t1204e859 = { $kind } di { $url }

# for {method} {url}
t093a86f6 = untuk { $method } { $url }

# The request, before the server gets it
tc840fed6 = Permintaan, sebelum diterima server

# The response, before the app gets it
t6683d72a = Respons, sebelum diterima aplikasi

# A socket's message, before the server gets it
tda2ebc5d = Pesan soket, sebelum diterima server

# A socket's message, before the app gets it
tbce07e9d = Pesan soket, sebelum diterima aplikasi

# Message
tae0ed984 = Pesan

# Send as it came
tb593e730 = Kirim apa adanya

# Drop
t108524ac = Buang

# Paused
tde281feb = Dijeda

# Nothing is waiting.
tea613769 = Tidak ada yang menunggu.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
       *[other] { $n } menunggu diteruskan.
    }

# Send all as they came
tdce25640 = Kirim semua apa adanya

# The list was not saved: {e}
tfceba380 = Daftar tidak tersimpan: { $e }

# {count} service: {names} | {count} services: {names}
tc1b8764c = { $count } service: { $names }

# Add
t9dc3aa14 = Tambah

# https://api.example.com or http://localhost:50051
t14f9744c = https://api.example.com atau http://localhost:50051

# Reading…
t6aad8485 = Membaca…

# {count} file | {count} files
tde674bd8 = { $count } file

# {count} message | {count} messages
t8a1cd2f6 = { $count } message

# {count} service | {count} services
t51f71902 = { $count } service

# and {count} more problem | and {count} more problems
t591ce5ea = dan { $count } masalah lain

# Remove
t21a5901d = Hapus

# Protobuf schemas
ta4401d1a = Skema Protobuf

# Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
te5f4f07c = Folder berisi file .proto, atau descriptor set yang dibuat dengan protoc -o, untuk menampilkan body Protobuf dan gRPC dengan nama field-nya. Panggilan gRPC menemukan message-nya dari path; Protobuf lain dari messageType yang disebut tipenya. Dibaca ulang saat berubah.

# Add a folder…
tb5930e43 = Tambah folder…

# Add files…
ted08dbe5 = Tambah file…

# Read again
t2f0dcc5d = Baca ulang

# Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.
tf6948a5a = Atau minta file dari server yang menyediakan gRPC reflection. https:// lewat tracewing; http:// adalah gRPC tanpa TLS, langsung ke server.

# Asking…
tf9cf22fe = Meminta…

# Fetch
t533a3183 = Ambil

# Find a message
te6f78626 = Cari message

# Read as {message}
t87b6a380 = Baca sebagai { $message }

# Read as…
t1b90af95 = Baca sebagai…

# Requests to {path} on {host} are read as the message chosen.
t8826d766 = Request ke { $path } di { $host } dibaca sebagai message yang dipilih.

# Answers from {path} on {host} are read as the message chosen.
tbbe012da = Jawaban dari { $path } di { $host } dibaca sebagai message yang dipilih.

# Automatic (gRPC path or messageType)
t134c9a06 = Otomatis (path gRPC atau messageType)

# and {count} more: type to narrow them | and {count} more: type to narrow them
t70ff68f9 = dan { $count } lagi: ketik untuk mempersempit

# Write the server's address first
t4f68f91a = Tulis alamat server dulu

# "{address}" is not a server's address
t8c4d03a6 = "{ $address }" bukan alamat server

# The server has no services to describe
tfa5b7ad8 = Server tidak punya service untuk dijelaskan

# The server sent no files for its services
tdca19bdc = Server tidak mengirim file untuk service-nya

# The server answered {status}: it may not offer gRPC reflection
t345fdb40 = Server menjawab { $status }: mungkin tidak menyediakan gRPC reflection

# The server answered {code}: it may not offer gRPC reflection. {message}
tb629329e = Server menjawab { $code }: mungkin tidak menyediakan gRPC reflection. { $message }

# The server answered nothing: it may not offer gRPC reflection
t6bbb6207 = Server tidak menjawab apa-apa: mungkin tidak menyediakan gRPC reflection

# The password is kept in the system's keychain.
t3a48373c = Kata sandi disimpan di keychain sistem.

# The password is kept in relay.toml, which only you can read.
tf84cb18b = Kata sandi disimpan di relay.toml, yang hanya bisa Anda baca.

# The password is kept in relay.toml, which only you can read: the keychain cannot be used ({why}).
t5a08b6d0 = Kata sandi disimpan di relay.toml, yang hanya bisa Anda baca: keychain tidak bisa dipakai ({ $why }).

# New connections go through the {kind} proxy {address}.
tb5f1ef96 = Koneksi baru melewati proxy { $kind } { $address }.

# Connections go straight to servers.
t4a64a08f = Koneksi langsung ke server.

# relay.toml cannot be used, so connections go straight: {problem}
tbc6a2da3 = relay.toml tidak bisa dipakai, jadi koneksi langsung: { $problem }

# http://proxy.example.com:8080 or socks5://proxy.example.com:1080
tb945a053 = http://proxy.example.com:8080 atau socks5://proxy.example.com:1080

# Go through another proxy
tc67dfdb1 = Lewat proxy lain

# Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080; the password is then kept in the system's keychain, not shown again. Empty, tracewing goes straight to servers.
t5c30a22f = Jika jaringan perusahaan hanya mengizinkan keluar lewat proxy-nya sendiri, tracewing meneruskan ke server lewat proxy itu. Aplikasi tetap mengirim traffic ke tracewing. Nama pengguna dan kata sandi ditulis di alamat, seperti http://nama:sandi@proxy:8080; kata sandi lalu disimpan di keychain sistem dan tidak ditampilkan lagi. Jika kosong, tracewing langsung ke server.

# Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host.
tcbc7d298 = Host yang dihubungi langsung, seperti aturan NO_PROXY: example.com berarti host itu dan semua di bawahnya, * berarti semua host.

# Sent by the server, not asked for
td717c770 = Dikirim server, bukan jawaban atas permintaan

# any
t2c29f04d = apa saja

# Failed
t5579c3d4 = Gagal

# The server answered 200: a call that fails says so in the body.
ta74e5f73 = Server menjawab 200: panggilan yang gagal menyatakannya di body.

# {count} item | {count} items
t534e422e = { $count } item

# {count} field | {count} fields
t0039810a = { $count } bidang

# and {count} more
tbdb1ff6a = dan { $count } lagi

# {count} byte that does not read as one | {count} bytes that do not read as one
ta7ee514c = { $count } byte yang tidak terbaca sebagai transaksi

# {program}, {count} bytes
t1ce6ad3b = { $program }, { $count } byte

# succeeded
t6b166ff2 = berhasil

# failed: it was mined, and what it asked for did not happen
t46ea918c = gagal: sudah masuk blok, dan yang dimintanya tidak terjadi

# none: it makes a contract
t188b58f6 = tidak ada: ini membuat kontrak

# {count} byte, which the signature does not say how to read | {count} bytes, which the signature does not say how to read
t36ffce30 = { $count } byte yang tidak dijelaskan cara membacanya oleh signature

# {count} place | {count} places
t0965779e = { $count } tempat

# {count} blob | {count} blobs
t95369f92 = { $count } blob

# 0, the kind from before EIP-2718
tc2fde48e = 0, jenis dari sebelum EIP-2718

# The bodies were not kept, so what the call sent cannot be shown.
teaf455ca = Body tidak disimpan, jadi apa yang dikirim panggilan ini tidak bisa ditampilkan.

# and {count} more calls
tf0ac7103 = dan { $count } panggilan lagi

# {count} function | {count} functions
t9ffc3c7c = { $count } fungsi

# {count} event | {count} events
t4dd03992 = { $count } event

# {count} error | {count} errors
t4fba4c4e = { $count } error

# Contract ABIs
t08fb00f2 = ABI kontrak

# Folders or files of ABI JSON, as solc, Hardhat and Foundry write it, to name the functions, events and errors in a JSON-RPC call instead of showing four bytes. They are read again when they change. Nothing is sent anywhere: a signature is worked out from the ABI on this machine.
tfadffbd2 = Folder atau file ABI JSON, seperti yang ditulis solc, Hardhat, dan Foundry, untuk menyebut nama fungsi, event, dan error dalam panggilan JSON-RPC alih-alih menampilkan empat byte. Dibaca ulang saat berubah. Tidak ada yang dikirim ke mana pun: signature dihitung dari ABI di mesin ini.

# Block
tbd074002 = Blokir

# Answer from a file
td8edab50 = Jawab dari file

# Send to another host
te3cd85d7 = Kirim ke host lain

# Set a request header
tfad4aaaa = Set header permintaan

# Set a response header
tcbbc20d0 = Set header respons

# Remove a request header
t5228dd84 = Hapus header permintaan

# Remove a response header
teeb5735a = Hapus header respons

# Set a query parameter
t73d9419b = Set parameter query

# Remove a query parameter
t12633da9 = Hapus parameter query

# Change the request body
t41101201 = Ubah body permintaan

# Change the response body
tdd259b73 = Ubah body respons

# Change the status
t4bbdfba8 = Ubah status

# Pause the request
t40ecc39b = Jeda permintaan

# Pause the response
t18a34799 = Jeda respons

# Pause a socket's messages to the server
te5d065fe = Jeda pesan soket ke server

# Pause a socket's messages to the app
t482173e8 = Jeda pesan soket ke aplikasi

# Status to answer with, like 403
t8f3ebb6c = Status untuk menjawab, seperti 403

# Full path of a file or folder
t9fb9ae4c = Path lengkap file atau folder

# Where to, like http://localhost:3000
tdfedd004 = Tujuan, seperti http://localhost:3000

# Header's name
t80793085 = Nama header

# Its value
t43968778 = Nilainya

# Parameter's name
t313c8e35 = Nama parameter

# Text to find, or empty for the whole body
t8255fc87 = Teks yang dicari, atau kosong untuk seluruh body

# What to put in its place
t03671fd5 = Penggantinya

# Status the client gets, like 500
t5e4d3088 = Status yang diterima klien, misalnya 500

# "{first}" is not a status
td43128d3 = "{ $first }" bukan status

# Block with {status}
t1727e984 = Blokir dengan { $status }

# Answer from {path}
taefc08fa = Jawab dari { $path }

# Send to {to}
t5e256873 = Kirim ke { $to }

# Request: {header}: {value}
taa230766 = Permintaan: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Respons: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Permintaan: tanpa { $header }

# Response: no {header}
t23960a2e = Respons: tanpa { $header }

# Query: {name}={value}
t0ee6c1c0 = Query: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Query: tanpa { $name }

# Status {status}
tad048e53 = Status { $status }

# Request body: {replace}
te4856c36 = Body permintaan: { $replace }

# Response body: {replace}
t8600dc42 = Body respons: { $replace }

# Request body: {find} → {replace}
taec471a9 = Body permintaan: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Body respons: { $find } → { $replace }

# Name
t0fe07306 = Nama

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, seperti api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Metode, atau kosong untuk semua

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
       *[other] { $n } aturan ditulis ke { $path }
    }

# The rules were not written: {e}
te7ac257f = Aturan tidak ditulis: { $e }

# Import
t573e0af4 = Impor

# {path} could not be read: {e}
t32a3ce94 = { $path } tidak bisa dibaca: { $e }

# Nothing was imported: {e}
tb9f7257a = Tidak ada yang diimpor: { $e }

# Edit
tc2c76cb1 = Edit

# Delete
t5797ea6a = Hapus

# Change this rule
t9160f393 = Ubah aturan ini

# A new rule
t55a722a0 = Aturan baru

# It waits under Paused until you send it on, changed or not.
tca566a57 = Permintaan menunggu di Dijeda sampai Anda meneruskannya, diubah atau tidak.

# Save
t4d2d5d68 = Simpan

# Cancel
t35afca3b = Batal

# Rules
t428b2d80 = Aturan

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Aturan berlaku mulai permintaan berikutnya, juga saat perekaman dijeda. Dari atas ke bawah: blokir, file, atau host pertama yang cocok menang, dan setiap aturan lain yang cocok berlaku, berurutan. Aturan yang mengubah body atau status mengubah apa yang dikirim server.

# Import rules…
t5edc205d = Impor aturan…

# Export rules…
t6e09ca20 = Ekspor aturan…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
       *[other] { $n } aturan ditambahkan
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
       *[other] { $n } sudah ada
    }

# left out: {why}
ta81e95e2 = dilewati: { $why }

# Copy all
t6aef5c37 = Salin semua

# Too large to copy all
t2fe8ff9f = Terlalu besar untuk disalin semua

# No flows
t6dc463eb = Tidak ada flow

# Name it to keep it
t52c1a402 = Beri nama agar disimpan

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · ditulis oleh versi lain, jadi tidak bisa dibuka

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
       *[other] { $count } flow · { $size }
    }

# Recording now
t3cb3e6fc = Sedang merekam

# Export HAR
t4d6458cc = Ekspor HAR

# Delete for good?
ta7737f13 = Hapus permanen?

# Sessions
tb69a8aec = Sesi

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Sesi yang diberi nama disimpan sampai Anda menghapusnya. Dari yang lain, { $kept } yang terbaru dipertahankan.

# Working…
td5507854 = Memproses…

# Open a session file
tae8622fc = Buka file sesi

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, .saz Fiddler, atau .chlsj Charles

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = File HAR yang diekspor tidak menyertakan cookie, token, kunci, dan kata sandi, di header, URL, dan body

# Other devices
t9e7d8362 = Perangkat lain

# Let other devices use the proxy
t13e626b3 = Izinkan perangkat lain memakai proxy

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = Ponsel dan komputer lain di jaringan Anda lalu bisa mengirim lalu lintasnya ke sini. Begitu juga siapa pun di jaringan itu: matikan ini di jaringan yang tidak Anda percayai.

# Servers
tcd0cf113 = Server

# Trust what this computer trusts
tfea2d91b = Percayai apa yang dipercayai komputer ini

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = Server dengan sertifikat dari otoritas yang terpasang di komputer ini, seperti milik perusahaan sendiri, dijangkau seperti browser di komputer ini menjangkaunya. Jika mati, hanya otoritas publik yang dipercaya. Berlaku saat tracewing dimulai ulang.

# AI tools
t294d7ea4 = Alat AI

# Let AI tools see tokens and cookies
te1d26d3c = Izinkan alat AI melihat token dan cookie

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = Alat AI membaca lalu lintas yang direkam lewat server MCP tracewing. Jika mati, nilai Authorization, Cookie, dan Set-Cookie tidak disertakan dalam yang mereka terima. Langsung berlaku.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Jauhkan aplikasi dari HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = HTTP/3 berjalan di atas UDP, yang tidak dibawa proxy mana pun, jadi aplikasi yang memakainya melewati tracewing. Ini menghapus tawaran HTTP/3 dari respons server, sehingga aplikasi tetap di HTTP/2. Flow tetap menampilkan tawaran itu seperti yang dikirim server.

# Size of a session
ta20fa102 = Ukuran sesi

# Past this, the oldest flows are dropped.
td2914f46 = Lewat dari ini, flow terlama dibuang.

# Room to leave on the disk
t8674a3b3 = Ruang yang disisakan di disk

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Jika ruang kosong kurang dari ini, flow tetap didaftar tetapi body tidak disimpan.

# Earlier sessions to keep
td356194a = Sesi sebelumnya yang disimpan

# Older ones are deleted when a new session starts.
t66556513 = Yang lebih lama dihapus saat sesi baru dimulai.

# Bodies
tdb4d0409 = Body

# Most kept of one body
t777508dd = Maksimum yang disimpan dari satu body

# It still reaches the client whole.
t386fee28 = Body tetap sampai utuh ke client.

# Most unpacked for viewing
tc3fe2efd = Maksimum hasil dekompresi untuk ditampilkan

# Stops a small download that unpacks to many GB.
tf9c49515 = Mencegah unduhan kecil yang didekompresi menjadi berGB-GB.

# Small body size
ta8996af1 = Ukuran body kecil

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Body yang lebih kecil menunggu di memori dan berbagi satu file. Yang lebih besar mendapat file sendiri saat tiba.

# Compress plain text on disk
tee503c2e = Kompres teks biasa di disk

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Memakan sekitar separuh ruang, dan dilewati saat lalu lintas terlalu cepat.

# Compression level
td386c9ef = Tingkat kompresi

# 1 to 19. Higher is smaller and slower.
td42a987d = 1 sampai 19. Lebih tinggi berarti lebih kecil dan lebih lambat.

# Search
tc646a2c9 = Pencarian

# Search URLs
t0c3e0b3d = Cari URL

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Menemukan bagian mana pun dari URL saat Anda mengetik. Ringan.

# Search bodies
t8eec0293 = Cari body

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Menemukan kata di body. Bisa memakan disk sebanyak body itu sendiri.

# Searched part of each body
tab293b21 = Bagian body yang dicari

# Words past this are not found. More takes more disk.
t0bf65f4a = Kata setelah batas ini tidak ditemukan. Lebih besar memakan lebih banyak disk.

# Under heavy traffic
ta99888cb = Saat lalu lintas padat

# Memory for bodies waiting to be written
t7d6ff799 = Memori untuk body yang menunggu ditulis

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Lewat dari ini, flow didaftar tanpa body-nya. Lalu lintas tidak pernah ditahan.

# Memory for large bodies waiting to be written
t07b9de08 = Memori untuk body besar yang menunggu ditulis

# Past this, large bodies stop being kept.
tfe61a4f3 = Lewat dari ini, body besar tidak lagi disimpan.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }" bukan bilangan bulat

# {key}: must not be negative
t9c4debaa = { $key }: tidak boleh negatif

# After a restart
t58dcea2d = Setelah mulai ulang

# Every setting is back to its default.
t763785e7 = Semua setelan kembali ke bawaannya.

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d = { $n } perubahan berlaku saat tracewing dimulai ulang.

# Settings
t4b058728 = Setelan

# Undo
t71fd4acf = Urungkan

# Back to defaults
ta3162a13 = Kembalikan ke default

# Hosts not to decrypt
tb457f0b3 = Host yang tidak didekripsi

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = Satu per baris: host, atau *.example.com untuk host itu dan semua di bawahnya. Lalu lintasnya lewat tanpa dibaca. Daftar ini langsung berlaku.

# Authorities trusted for servers
t0a308a8a = Otoritas yang dipercaya untuk server

# tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in {folder}. They are read when tracewing starts.
t571051f9 = tracewing memercayai server yang dijamin otoritas publik, dan yang dijamin otoritas komputer ini. Ia juga memercayai yang berikut: otoritas milik perusahaan, atau milik Anda sendiri, sebagai file .pem di { $folder }. Dibaca saat tracewing dimulai.

# Left out: {why}
t4a567d82 = Dilewati: { $why }

# None yet.
t786bc275 = Belum ada.

# The folder has changed since tracewing started: restart it to use what it holds now.
t65f2d800 = Folder berubah sejak tracewing dimulai: mulai ulang untuk memakai isinya sekarang.

# Open the folder
tc7d7e212 = Buka folder

# Read it again
td72efd5e = Baca lagi

# TLS keys for Wireshark
tcb4394af = Kunci TLS untuk Wireshark

# Writes the secrets of each HTTPS connection, from apps to tracewing and from it to servers, to a file Wireshark reads: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Anyone with the file can read the traffic it covers. Off again when tracewing starts, unless SSLKEYLOGFILE names a file.
t2f92ac10 = Menulis rahasia setiap koneksi HTTPS, dari aplikasi ke tracewing dan dari tracewing ke server, ke file yang dibaca Wireshark: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Siapa pun yang memiliki file ini dapat membaca lalu lintas yang dicakupnya. Mati lagi saat tracewing dimulai, kecuali SSLKEYLOGFILE menunjuk sebuah file.

# Cannot write to it: {why}
t956ecc4b = Tidak dapat menulis ke sana: { $why }

# Copy the path
t095d064e = Salin jalur

# Language
t9a73db9b = Bahasa

# System ({system_name})
tb5c7a1d4 = Sistem ({ $system_name })

# Applies at once.
tefb7a0f9 = Langsung berlaku.

# Your browser
tb5a48dd2 = Browser Anda

# Its private key is in the system keychain.
t62a47808 = Kunci privatnya ada di keychain sistem.

# Its private key is in ca.key, which only you can read.
td9c6165d = Kunci privatnya ada di ca.key, yang hanya bisa dibaca oleh Anda.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = Kunci privatnya ada di ca.key, yang hanya bisa dibaca oleh Anda, karena keychain sistem tidak bisa menyimpannya: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Buka halamannya. Jika browser memperingatkan soal sertifikat, atau tidak menemukan halamannya, salah satu langkah di atas belum dilakukan untuk browser itu.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } lewat tracewing dan memercayainya.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } lewat tracewing, tetapi membuka halaman tanpa HTTPS. Buka lagi lewat https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = Ponsel atau komputer lain juga bisa mengirim lalu lintasnya ke sini. Aktifkan “Izinkan perangkat lain memakai proxy” di Setelan.

# this computer's address, port {port}
t2188b3f4 = alamat komputer ini, port { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = Di perangkat, atur proxy Wi-Fi ke { $addr }. Lalu buka http://{ $host } di sana, ambil sertifikatnya, dan percayai. Flow-nya menampilkan alamatnya, dan bisa difilter dengan device:. Beri nama perangkat dari salah satu flow-nya, di Koneksi.

# Looking for it…
t50ee7452 = Mencari…

# Installing. The system may ask you to confirm.
ta41c38ec = Memasang. Sistem mungkin meminta konfirmasi.

# Removing. The system may ask you to confirm.
t62b22726 = Menghapus. Sistem mungkin meminta konfirmasi.

# This computer trusts it.
t27d6bacc = Komputer ini memercayainya.

# Installed in: {places}.
tb5bcc244 = Terpasang di: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Alat baris perintah memercayainya, tetapi browser punya penyimpanan sendiri. Sampai sertifikat ada di sana, HTTPS lewat tracewing gagal di browser.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing membaca HTTPS dengan sertifikat yang dibuat khusus untuk komputer ini. Sampai sertifikat itu dipercaya, HTTPS lewat tracewing gagal.

# Trust the certificate
t26618699 = Percayai sertifikat

# Install
tb50cbd40 = Pasang

# Also for command-line tools
t0db8faea = Juga untuk alat baris perintah

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Hapus sertifikat dari semua penyimpanan di komputer ini? HTTPS melalui tracewing akan gagal sampai dipasang lagi. tracewing tetap menyimpannya untuk dipasang lagi nanti.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = Di Linux, browser butuh certutil agar bisa diminta memercayainya. Pasang certutil, lalu Pasang di sini:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = Di Linux, browser butuh certutil agar bisa diminta memercayainya. Pasang NSS tools dari distribusi Anda, lalu Pasang di sini.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = Desktop ini tidak punya pengaturan proxy sistem yang bisa diubah tracewing. Buka browser terpisah dari Browser, atau terminal, di bawah.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } tidak punya pengaturan proxy sistem yang bisa diubah tracewing. Buka browser terpisah dari Browser, atau terminal, di bawah.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Aplikasi yang memakai setelan sistem kini lewat { $addr }. Setelan kembali seperti semula saat tracewing ditutup, atau jika crash.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Mengirim lalu lintas komputer ini lewat tracewing, untuk aplikasi yang memakai setelan sistem. Atau arahkan sendiri satu aplikasi ke { $addr }.

# Turn on the system proxy
t52b88441 = Aktifkan proxy sistem

# Turn off
tcd03e04b = Matikan

# Turn on
t65a0c5a7 = Aktifkan

# Check it in your browser
tbed6e83a = Periksa di browser

# Open the test page
te4e36f65 = Buka halaman uji

# Set up
t43c06abc = Siapkan

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Tiga langkah untuk melihat lalu lintas HTTPS dari komputer ini.

# See traffic
t980eee79 = Lihat lalu lintas

# the browsers' own stores
tddf6836c = penyimpanan milik browser sendiri

# Trusts the certificate.
tc5bba858 = Memercayai sertifikat.

# Does not trust the certificate yet.
tf2a0b8c5 = Belum memercayai sertifikat.

# Looking…
t284579ba = Mencari…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Belum punya profil. Buka sekali, lalu cari lagi.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 = { $holding } dari { $n } profilnya memercayai sertifikat.

# Has not opened the test page yet.
t67f9cb49 = Belum membuka halaman uji.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Membuka halaman uji melalui tracewing, lewat HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Membuka halaman uji melalui tracewing, tetapi tanpa HTTPS.

# Browsers
td580efd4 = Browser

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Masing-masing punya aturan sendiri tentang apa yang dipercaya dan ke mana lalu lintasnya dikirim. Periksa membuka halaman uji di browser itu. Buka terpisah menjalankannya dengan profil sendiri yang lewat tracewing, tanpa menyentuh proxy sistem.

# Look again
t98416222 = Cari lagi

# No browser found on this computer.
t0e46bb34 = Tidak ada browser yang ditemukan di komputer ini.

# Check
t592c8de7 = Periksa

# Open on its own
t0da7783c = Buka terpisah

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = Port UDP 443 diblokir di komputer ini, jadi aplikasi turun dari HTTP/3 ke HTTP/2 dan lewat tracewing. Pemblokiran berhenti saat tracewing ditutup, atau jika crash.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Sebagian aplikasi memakai HTTP/3, lewat UDP, dan melewati proxy. Memblokir port UDP 443 di komputer ini membuatnya turun ke HTTP/2 lewat tracewing. Perlu kata sandi administrator, dan berhenti saat tracewing ditutup. VPN yang memakai port UDP 443 tidak berfungsi selama itu.

# Apps on HTTP/3
t2cfe5270 = Aplikasi di HTTP/3

# Stop blocking
t318358d8 = Berhenti memblokir

# Block HTTP/3
t88231e40 = Blokir HTTP/3

# Or one app at a time
t91c63f1c = Atau satu aplikasi saja

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Terminal yang lewat tracewing, sementara proxy sistem dan semua app lain dibiarkan apa adanya. Untuk browser, Buka terpisah di Browser melakukan hal yang sama dengan profil sendiri.

# Open a terminal
t4b629c72 = Buka terminal

# iPhone and iPad
ta550cc97 = iPhone dan iPad

# Flutter apps
ta04ee397 = Aplikasi Flutter

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. Di Pengaturan, Wi-Fi, pilih jaringan, lalu Konfigurasikan Proksi, Manual: server { $server }, port { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Pindai kode dengan kamera, atau buka { $page } di Safari, lalu ambil sertifikatnya. Izinkan unduhan.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. Di Pengaturan, Umum, VPN & Manajemen Perangkat, pilih profil tracewing CA dan instal.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. Di Pengaturan, Umum, Mengenai, Pengaturan Kepercayaan Sertifikat, aktifkan kepercayaan penuh untuk tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Buka https://{ $host } di Safari. Jika tertulis berhasil, aplikasi yang memakai pengaturan sistem lewat tracewing.

# When you are done
t62466f08 = Setelah selesai

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. Di Settings, Wi-Fi, pilih jaringan, lalu Configure Proxy: Off. Jika dibiarkan, perangkat tidak punya Internet di jaringan ini setelah tracewing ditutup.

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. Di Settings, General, VPN & Device Management, pilih tracewing CA dan hapus profilnya. Kepercayaan penuhnya ikut hilang.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. Di Setelan, Wi-Fi, edit jaringan, lalu di Opsi lanjutan atur proxy ke Manual: host { $host }, port { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Pindai kode, atau buka { $page } di browser, lalu ambil sertifikatnya.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. Di Setelan, Keamanan, Enkripsi & kredensial (namanya beda-beda tiap produsen), pilih Instal sertifikat, lalu Sertifikat CA, dan pilih tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Sejak Android 7, aplikasi hanya memercayai sertifikat yang ditambahkan pengguna jika menyatakannya. Untuk build debug aplikasi Anda sendiri, tambahkan res/xml/network_security_config.xml:

# and point to it from AndroidManifest.xml:
t02ad844b = lalu rujuk dari AndroidManifest.xml:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Aplikasi lain tetap menolak sertifikat: lalu lintasnya muncul sebagai flow yang gagal.

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. Di Settings, Wi-Fi, ubah jaringan, dan kembalikan proxy ke None. Jika dibiarkan, perangkat tidak punya Internet di jaringan ini setelah tracewing ditutup.

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. Di Settings, Security, Encryption & credentials, User credentials, pilih tracewing dan hapus. Hapus juga tracewing.crt dari Downloads.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = HttpClient milik Dart, yang dipakai http dan Dio di Flutter, tidak mengikuti setelan proxy ponsel, dan di Android tidak memercayai sertifikat yang ditambahkan pengguna. Di build debug, arahkan ke sini:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback meloloskan sertifikat apa pun, karena itu hanya untuk build debug.

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = Hapus TracewingProxy dan barisnya di main() sebelum rilis, meski keduanya tidak berbuat apa-apa di luar build debug. Tidak ada yang diubah di ponsel.

# Open settings
t1336b4d6 = Buka setelan

# Phones and other devices
t2e3c591c = Ponsel dan perangkat lain

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Ponsel, tablet, atau komputer lain di jaringan ini bisa lewat tracewing di { $proxy }. Kode ini membuka halaman berisi sertifikatnya. Flow-nya menampilkan alamatnya, bisa difilter dengan device:, dan perangkat bisa diberi nama dari salah satu flow-nya, di Koneksi.

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = Perangkat lain tidak bisa menjangkau tracewing di salah satu alamat komputer ini: { $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f = { $n } perangkat lewat tracewing ({ $names }). Setelah selesai, matikan proxy Wi-Fi-nya seperti di bawah: jika dibiarkan, perangkat tidak punya Internet di jaringan ini setelah tracewing ditutup.

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Siap. Sertifikat ada di antara sertifikat sistem sampai perangkat dimulai ulang.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Siap. Sertifikat ada di Download: di Setelan yang baru dibuka, pilih Enkripsi & kredensial, Instal sertifikat, Sertifikat CA, lalu pilih tracewing.crt.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Aplikasi yang hanya memercayai sertifikat sistem tetap akan menolaknya.

# An emulator.
t14b6b86d = Emulator.

# On USB.
t67dafc14 = Lewat USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Terima permintaan debug USB di perangkat, lalu cari lagi.

# Still starting. Look again in a moment.
t13f68b64 = Masih memulai. Cari lagi sebentar lagi.

# adb says it is {state}.
t2b7d71da = adb melaporkan statusnya { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Mengatur proxy di emulator Android, atau ponsel yang tersambung dengan debug USB aktif, dan memasang sertifikat. Lalu lintas datang lewat ADB, jadi keduanya tidak perlu berada di jaringan ini.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = adb tidak ditemukan. Instal Android Studio, atau platform tools Android, lalu cari lagi.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Tidak ada perangkat yang tersambung. Jalankan emulator, atau colokkan ponsel dengan debug USB aktif, lalu cari lagi.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Siapkan perangkat agar mengirim lalu lintasnya ke sini. Urungkan, atau menutup tracewing, akan melepas proxy darinya lagi.

# Android emulators and USB devices
t379e433c = Emulator Android dan perangkat USB

# Find devices
t54d4af39 = Temukan perangkat

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = Proxy sistem dan pemblokiran HTTP/3 kembali seperti semula saat tracewing ditutup atau crash, begitu juga perangkat Android yang disiapkan lewat USB. Sertifikat tetap dipercaya sampai tombol Hapus di langkah 1 mengeluarkannya. Ponsel yang disiapkan secara manual tetap menyimpan proxy Wi-Fi dan sertifikatnya sampai dihapus di ponsel itu: panduan tiap perangkat menjelaskan caranya. Perangkat lain bisa memakai proxy komputer ini sampai itu dimatikan di Settings.

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code, dan alat AI lain bisa mendaftar dan membaca flow yang direkam tracewing, lewat server MCP-nya. Token dan cookie tidak disertakan kecuali Setelan mengizinkannya. Di Claude Code:

# Elsewhere, in the tool's MCP settings:
tec38833c = Di tempat lain, di setelan MCP alat tersebut:

# Flows
t4db77db2 = Flow

# Choose the flow above or below
tecb3adec = Pilih flow di atas atau di bawah

# The first or the last flow
t7e278a00 = Flow pertama atau terakhir

# A page up or down
t23b1814b = Satu halaman ke atas atau ke bawah

# Filter
tf4a9c097 = Filter

# Pause or resume recording
t6e21519a = Jeda atau lanjutkan perekaman

# Clear the list
tf2a55d1f = Kosongkan daftar

# Compare the flow clicked with the chosen one
te42cd067 = Bandingkan flow yang diklik dengan yang dipilih

# The chosen flow
te0a729de = Flow yang dipilih

# Its request
tb3970784 = Permintaannya

# Its response
t221fc168 = Responsnya

# Send it again
tef7d1944 = Kirim ulang

# Edit it and send it
t6bc99e5c = Edit lalu kirim

# Copy it as cURL
tc99a11a3 = Salin sebagai cURL

# The part before or after the one shown
tdf4221d9 = Bagian sebelum atau sesudah yang ditampilkan

# Save the body shown
t20d9b7c0 = Simpan body yang ditampilkan

# Select all of the part shown
tf9168168 = Pilih semua di bagian yang ditampilkan

# Copy the selected text
t00b090d3 = Salin teks yang dipilih

# Send it on, as edited
t8c35fa5a = Kirim, sesuai suntingan

# Send it on as it was
t16f7abb5 = Kirim apa adanya

# Drop it
t4e7f48b9 = Buang

# The window
t2af18488 = Jendela

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Siapkan, Lalu Lintas, Aturan, Dijeda, Susun, Sesi, Setelan

# Show or hide the sidebar
t67adca04 = Tampilkan atau sembunyikan sidebar

# Show or hide the inspector
ta5a7b9e9 = Tampilkan atau sembunyikan inspektor

# The inspector beside the list or below it
t51e214f7 = Inspektor di samping daftar atau di bawahnya

# The next or the previous field
ta4b4bd5e = Kolom berikutnya atau sebelumnya

# These shortcuts
tbdf3b62b = Pintasan ini

# Close them
tf3a4b1ef = Tutup pintasan

# Keyboard shortcuts
t31eaddb7 = Pintasan keyboard

# Close
tcd86acc3 = Tutup

# The certificate: {e}
t0235648c = Sertifikat: { $e }

# The terminal
tf542b2f4 = Terminal

# The system proxy: {e}
tc99726ce = Proxy sistem: { $e }

# Blocking HTTP/3: {e}
t024ec871 = Memblokir HTTP/3: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx, atau teks di body

# A name, as iPhone test
t19f89214 = Nama, misalnya Tes iPhone

# A name, as Phone
tfce439bf = Nama, misalnya Phone

# A message to send; {keys} sends it
tf9d9c09b = Pesan untuk dikirim; { $keys } mengirimnya

# The search index: {e}
te3c971c9 = Indeks pencarian: { $e }

# Click the flow to compare #{id} with.
tac1ec1d2 = Klik flow untuk dibandingkan dengan #{ $id }.

# An earlier session: {name}
t73446a1d = Sesi sebelumnya: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Hanya-baca. Pencarian mencakup lebih banyak seiring indeksnya selesai. Perekaman tetap berjalan.

# Read-only. Recording goes on.
te2f35d9e = Hanya-baca. Perekaman tetap berjalan.

# Back to live
t317c0453 = Kembali ke live

# {shown} of {total}
tf2cc0563 = { $shown } dari { $total }

# Recording
te75cf20c = Merekam

# System proxy off · {addr}
t7e813d25 = Proxy sistem mati · { $addr }

# System proxy…
t29e7dcd0 = Proxy sistem…

# System proxy on · {addr}
t319ad6b9 = Proxy sistem aktif · { $addr }

# Apps
t53d91aad = Aplikasi

# Devices
t27acf320 = Perangkat

# All hosts
tfc7bfe37 = Semua host

# No host has this in its name
t838c1d44 = Tidak ada host yang namanya memuat ini

# Hidden
t3c2dfe59 = Tersembunyi

# Hosts
tcf9aefb4 = Host

# Newest
t324b6d0b = Terbaru

# No flow fits this filter
t7eb295f4 = Tidak ada flow yang cocok dengan filter ini

# Nothing yet
tbd00747c = Belum ada apa-apa

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Aktifkan proxy sistem di atas, atau arahkan aplikasi ke { $addr }. Untuk mencoba: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Di samping daftar

# Below the list
t23658daf = Di bawah daftar

# Drop it on a place · Esc puts it back
t0fa632ba = Lepas di suatu tempat · Esc mengembalikannya

# Traffic
t4b589266 = Lalu Lintas

# Certificate trusted on this computer
t7b93f551 = Sertifikat dipercaya di komputer ini

# Checking the certificate…
tbf8942c3 = Memeriksa sertifikat…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Memasang sertifikat; sistem mungkin meminta konfirmasi…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Menghapus sertifikat; sistem mungkin meminta konfirmasi…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS akan gagal sampai sertifikat dipercaya: siapkan

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml tidak dipakai: { $problem }

# A setting was not saved: {problem}
tff1a5018 = Satu setelan tidak disimpan: { $problem }

# The proxy stopped: {failure}
t04dcff11 = Proxy berhenti: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = Disk hampir penuh: body tidak disimpan

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
       *[other] { $count } flow tidak direkam
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
       *[other] { $count } flow tanpa body
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
       *[other] { $count } flow terlama dibuang agar tetap di bawah { $size }
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
       *[other] { $count } flow
    }

# Export as HAR
t95b6b766 = Ekspor sebagai HAR

# Shortcuts {keys}
t4ab0ecb4 = Pintasan { $keys }

# Columns
tea156fdc = Kolom
