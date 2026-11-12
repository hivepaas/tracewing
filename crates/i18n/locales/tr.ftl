# tracewing in Turkish. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 =
    { $n ->
        [one] … { $count } satır { $closer }
       *[other] … { $count } satır { $closer }
    }

# … {size} more
ta8dc6eab = … { $size } daha

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc =
    { $n ->
        [one] { $size } { $format } · { $count } satır JSON
       *[other] { $size } { $format } · { $count } satır JSON
    }

# The first {count} rows are shown.
t16aa1c6d = Yalnızca ilk { $count } satır gösteriliyor.

# Unfold all
t3849d44e = Tümünü aç

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } satır
       *[other] { $size } · { $count } satır
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · okunuyor %{ $percent }

# Appearance: {now}. Click for {next}.
t332cfa6d = Görünüm: { $now }. Değiştirmek için tıklayın: { $next }.

# Method
t98a2c750 = Metot

# Status
t005ef20f = Durum

# App
t81f51acc = Uygulama

# Host
t302af18f = Host

# Path
teb66e456 = Yol

# Operation
t218e02a6 = İşlem

# Size
ta6478e7c = Boyut

# Time
tdfe4e404 = Süre

# Number
t61e9eac0 = Numara

# A body was not kept, so the two cannot be compared.
t9d6abb2a = Bir gövde saklanmadığı için ikisi karşılaştırılamıyor.

# {count} field differs | {count} fields differ
t6a8fff7f =
    { $n ->
        [one] { $count } alan farklı
       *[other] { $count } alan farklı
    }

# … {count} line the same | … {count} lines the same
tcc37e7ea =
    { $n ->
        [one] … { $count } aynı satır
       *[other] … { $count } aynı satır
    }

# Nothing to compare.
t9f32d942 = Karşılaştırılacak bir şey yok.

# URL
t7569633e = URL

# Protocol
t4a57ebd5 = Protokol

# Type
td155d06d = Tür

# Sizes
td1a4a19d = Boyutlar

# Body
t428a1095 = Gövde

# Headers
t334f5989 = Başlıklar

# Query
t9e89e783 = Sorgu

# Request
t82454152 = İstek

# Response
t55a982fe = Yanıt

# Neither has headers kept.
tb43ef094 = İkisinde de saklanan başlık yok.

# Trailers
t7c44fa89 = Trailer'lar

# Neither URL has a query.
t4d8203e0 = İki URL'de de sorgu yok.

# Comparing…
ta8565d31 = Karşılaştırılıyor…

# No body
tfa4c2902 = Gövde yok

# {kind}, its first {size} compared
t1d894c39 = { $kind }, ilk { $size } karşılaştırıldı

# Neither has a body.
tf2c47725 = İkisinde de gövde yok.

# The bodies are the same.
t85152e01 = Gövdeler aynı.

# {count} field differs. | {count} fields differ.
t0f651f77 =
    { $n ->
        [one] { $count } alan farklı.
       *[other] { $count } alan farklı.
    }

# The first {count} are listed.
taae62a3e = Yalnızca ilk { $count } tanesi listelendi.

# {count} line differs. | {count} lines differ.
tf57a3157 =
    { $n ->
        [one] { $count } satır farklı.
       *[other] { $count } satır farklı.
    }

# Fields
t3308bd9c = Alanlar

# Lines
t8a1a00dc = Satırlar

# Compare
td57a0a54 = Karşılaştır

# #{a} beside #{b}: what only #{a} has is red, what only #{b} has is green.
t71ce2de9 = #{ $a } ile #{ $b } yan yana: yalnızca #{ $a }'da olan kırmızı, yalnızca #{ $b }'de olan yeşil.

# Swap
te519d36e = Yer değiştir

# Back to Traffic
t644042d2 = Traffic'e dön

# "{method}" is not a method
tacdcf5cb = "{ $method }" bir metot değil

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = istek gövdesi bütün olarak saklanmadı, bu yüzden yeniden gönderilemez

# name: value, one a line
t4df37bea = ad: değer, satır başına bir tane

# No flow is chosen
t550b0a98 = Seçili akış yok

# This flow: {e}
t296762f8 = Bu akış: { $e }

# The body is not a {message}: {e}
t3220a10a = Gövde bir { $message } değil: { $e }

# Protobuf {message}, written as JSON: it is sent as Protobuf.
t83b6cd6e = Protobuf { $message }, JSON olarak yazılır: Protobuf olarak gönderilir.

# Sending…
te946e7bf = Gönderiliyor…

# Send
tf28e14cf = Gönder

# New
t8b242db1 = Yeni

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … { $count } bayt daha: Trafik'teki akışta hepsi var.

# Body · {message}
taa3cc3ff = Gövde · { $message }

# Compose
t18049f93 = Oluştur

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Kendi isteğiniz ya da yeniden gönderilen bir istek. tracewing üzerinden geçer, bu yüzden kurallar uygulanır ve Trafik'te de görünür.

# {version} both sides
t961a5ea0 = { $version }, iki tarafta da

# {client} to the proxy, {server} to the server
t3d237646 = proxy'ye { $client }, sunucuya { $server }

# Queued
t9e437c74 = Sırada

# Connect
tdb89a899 = Bağlantı

# Waiting
tc9450e48 = Bekleme

# Download
t6964bb19 = İndirme

# No file picker: {e}
t6935fbd3 = Dosya seçici yok: { $e }

# Writing the HAR…
t725a0e89 = HAR yazılıyor…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
        [one] { $n } akış şuraya yazıldı: { $path }
       *[other] { $n } akış şuraya yazıldı: { $path }
    }

# The HAR was not written: {e}
tef8ec569 = HAR yazılamadı: { $e }

# Open
t538b10e9 = Aç

# Reading {path}…
tc0e7c8f4 = Okunuyor: { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
        [one] , { $n } kayıt dışarıda bırakıldı
       *[other] , { $n } kayıt dışarıda bırakıldı
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
        [one] { $n } akış okundu ({ $format })
       *[other] { $n } akış okundu ({ $format })
    }

# The file was not read: {e}
t7f49510f = Dosya okunamadı: { $e }

# it is no longer kept
ta7b4c380 = artık saklanmıyor

# {size} saved to {path}
tc6932467 = { $size } kaydedildi: { $path }

# The body was not saved: {e}
t189900b4 = Gövde kaydedilemedi: { $e }

# Fiddler session
t9f5c89a3 = Fiddler oturumu

# Charles session
tb4f56f2b = Charles oturumu

# All
t75afe364 = Tümü

# Errors
te9ed60a6 = Hatalar

# Media
td10fcaa3 = Medya

# Rule: {rule}
teca2074f = Kural: { $rule }

# Events
t003420a4 = Olaylar

# JSON lines
te8ce2db0 = JSON satırları

# Not decrypted
t8090a1ba = Şifresi çözülmedi

# open
td35ec4c9 = açık

# Reading the request…
t51c96267 = İstek okunuyor…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = İsteğin gövdesi saklanmadı, bu yüzden sorgusu gösterilemiyor.

# An operation without a name
t38c4d30b = Adı olmayan bir işlem

# {n} of {count}: {name}
t53045460 = { $count } içinden { $n }: { $name }

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Persisted query: metnini sunucu tutar, istek yalnızca hash'ini gönderir.

# Variables
t7996f062 = Değişkenler

# Cut
t58e25863 = Kes

# Copy
t658f3664 = Kopyala

# Paste
tb34a0288 = Yapıştır

# Select all
t6af21072 = Tümünü seç

# This flow is gone.
t3343999e = Bu akış artık yok.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Bu host'un şifresi çözülmüyor, bu yüzden gönderdikleri görülemez.

# Still coming: it shows once it ends.
t42be95fb = Hâlâ geliyor: bittiğinde görünür.

# No body.
t31e4d044 = Gövde yok.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Bu gövde saklanmadı: disk neredeyse doluydu ya da sırada çok fazla veri bekliyordu.

# This body was not kept.
te46616b5 = Bu gövde saklanmadı.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Bu gövde artık yok: en eski akışlarla birlikte atıldı ya da okunamıyor.

# Cannot open this body: {e}
t620a039b = Bu gövde açılamıyor: { $e }

# Inspector
t4def410e = İnceleyici

# Choose a flow to see it here
tc7be91e8 = Burada görmek için bir akış seçin

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Şifresi çözülmedi. Bu host'a yeni bağlantılar okunmadan geçer.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Bir uygulama bu host'la başarısız oluyorsa, sertifika sabitleme (pinning) yapıyor olabilir.

# Make a rule
t0ac0e062 = Kural oluştur

# Decrypt this host
tb7367e2c = Bu host'un şifresini çöz

# Do not decrypt this host
t5fa113f6 = Bu host'un şifresini çözme

# Request and response
t04e060e6 = İstek ve yanıt

# Request and response one at a time
t70397119 = İstek ve yanıt tek tek

# Request and response side by side
t3d35ce69 = İstek ve yanıt yan yana

# Request above the response
t83c63123 = İstek yanıtın üstünde

# Table
td70ceb7f = Tablo

# Pretty
t7e09470d = Düzenli

# Raw
ta689aa91 = Ham

# Picture · {size}
te97ed633 = Resim · { $size }

# Messages {count}
te61b841a = Mesajlar { $count }

# Headers {headers}
t04b0146d = Başlıklar { $headers }

# Cookies {cookies}
td58eb84b = Çerezler { $cookies }

# Query {query}
t4756f9f5 = Sorgu { $query }

# Form {form}
tfa6e7203 = Form { $form }

# Connection
t09ea4ff9 = Bağlantı

# Save body
te2c44424 = Gövdeyi kaydet

# Send again
tf5910557 = Yeniden gönder

# Edit and send
tf837e542 = Düzenle ve gönder

# Copied
t8dc21305 = Kopyalandı

# Copy as cURL
t866d1e3e = cURL olarak kopyala

# Click another flow…
t59253578 = Başka bir akışa tıklayın…

# Compare with…
t58dfe040 = Şununla karşılaştır…

# Compare with the call before
tef15286c = Önceki çağrıyla karşılaştır

# Copy as…
t9c1ad448 = Şu olarak kopyala…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Bu resim çizilemiyor. Görmek için gövdeyi kaydedin.

# Opening…
tc40f2321 = Açılıyor…

# No response
t8c2c37b1 = Yanıt yok

# Tunnel, not decrypted
tce693254 = Tünel, şifresi çözülmedi

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# RPC: {method}
tf8162891 = RPC: { $method }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Daha önce kurulmuş { $id } bağlantısı üzerinden: bağlanmaya zaman harcanmadı.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Bulunamadı: sonlanmış ya da başka bir kullanıcıya ait olabilir

# On another device
t2b964a37 = Başka bir cihazda

# CLIENT
t6d3cd99e = İSTEMCİ

# Device
tbdcbb413 = Cihaz

# Program
tc4253b6b = Program

# Address
t700560f3 = Adres

# SERVER
te3b27612 = SUNUCU

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = Bu bağlantının ne olduğu artık saklanmıyor: daha eski akışlarla birlikte gitti.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Şifresi çözülmedi: tracewing bakmadan iletti.

# No connection was made.
tf348eb92 = Bağlantı kurulmadı.

# None: it was answered without one.
tac71808e = Yok: bağlantı olmadan yanıtlandı.

# {id}, given up during TLS
t0ec66a8c = { $id }, TLS sırasında vazgeçildi

# {id}, made for this flow
t7fe32cfc = { $id }, bu akış için kuruldu

# {id}, made before and used again
ta904c078 = { $id }, daha önce kuruldu ve yeniden kullanıldı

# Server
t6d7302f2 = Sunucu

# None: plain HTTP
t05606349 = Yok: düz HTTP

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = SUNUCU HİÇ SERTİFİKA GÖSTERMEDİ

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = SUNUCUNUN GÖSTERDİKLERİ, ÖNCE KENDİNİNKİ

# NO CERTIFICATES
te6f31dce = SERTİFİKA YOK

# CERTIFICATE
t1c902790 = SERTİFİKA

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
        [one] { $n } SERTİFİKA, ÖNCE SUNUCUNUNKİ
       *[other] { $n } SERTİFİKA, ÖNCE SUNUCUNUNKİ
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing bu sunucuyla devam etmedi: { $error }. Uygulama ona şirket CA'sı gibi başka bir yolla güveniyorsa “Bu host'un şifresini çözme”yi seçin; uygulama sunucuyu kendisi denetler.

# Cipher
t5b3bed6a = Şifre takımı

# none
tada7afdb = yok

# not valid until {when}
t0a14b2d8 = henüz geçerli değil: { $when }

# expired {when}
t932538c2 = süresi doldu: { $when }

# expires {when}
t679bcb51 = süresi doluyor: { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
        [one] { $names } ve { $n } tane daha
       *[other] { $names } ve { $n } tane daha
    }

# Issued by {issuer}
t15a1ce5a = Veren: { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Geçerli: { $from } – { $to } · { $when }

# Names {names}
tda75e459 = Adlar { $names }

# Serial {serial}
td1f0d98b = Seri { $serial }

# These headers were not kept.
tb8739fec = Bu başlıklar saklanmadı.

# No headers.
tf3dacb5e = Başlık yok.

# Trailers, sent after the body
t3447e9de = Gövdeden sonra gönderilen trailer'lar

# File {filename} · {kind} · {size}
t6dbd239f = Dosya { $filename } · { $kind } · { $size }

# no type
t902a6ece = tür yok

# No cookies were sent.
t58e85ab2 = Hiç çerez gönderilmedi.

# No cookies were set.
tf57dbabc = Hiç çerez ayarlanmadı.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · imza denetlenmedi

# JWT · signature not checked
t05f8b533 = JWT · imza denetlenmedi

# Header
t114f9980 = Başlık

# Payload
ta0639745 = Payload

# Expires
tc7aa0523 = Bitiş

# Issued
tb1835ecc = Veriliş

# Not before
td10dacd9 = Geçerlilik başlangıcı

# in {amount}
tfda57c3e = { $amount } sonra

# {amount} ago
t5f968014 = { $amount } önce

# Light
t8ad2f8cf = Açık

# Dark
t45570065 = Koyu

# System
t8f3184bc = Sistem

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
        [one] { $count } parça hâlinde aktarıldı
       *[other] { $count } parça hâlinde aktarıldı
    }

# stopped: {stop}
tfa48fb30 = durdu: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = token: { $input } giriş · { $output } çıkış

# Reading the call…
t33b52a3c = Çağrı okunuyor…

# Answer
tf94ea0ff = Cevap

# Thinking: {thinking}
t737ece33 = Düşünme: { $thinking }

# Calls {name}{id}
t017882e4 = Çağrı: { $name }{ $id }

# Nothing yet.
td9b71d16 = Henüz bir şey yok.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
        [one] { $text }… ve { $count } karakter daha
       *[other] { $text }… ve { $count } karakter daha
    }

# Copy the answer
t69975ed7 = Cevabı kopyala

# Asked
t4e3af9c3 = Soru

# Tools offered: {tools}
tb871bcf4 = Sunulan araçlar: { $tools }

# Not kept
tba30b5fa = Saklanmadı

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Bu mesaj saklanmadı: sırada çok fazla veri bekliyordu ya da disk neredeyse doluydu.

# This message is empty.
tdb0ec01f = Bu mesaj boş.

# No messages yet
t51e7624c = Henüz mesaj yok

# {said} kept, of {size}
t3558745a = { $size } içinden { $said } saklandı

# Layouts
t17809004 = Düzenler

# Keep the panels as they are now under a name, to come back to them.
t2b6aeb69 = Panellerin şu anki düzenini bir adla saklayın, sonra ona dönün.

# Keep
t555cbe68 = Sakla

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } bir tam sayı değil

# Latency
t23a45ef9 = Gecikme

# Down
ta4995035 = İndirme

# Up
t42f35940 = Yükleme

# Loss
t69db303a = Kayıp

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Yavaşlatmak için bir gecikme, hız ya da kayıp girin

# Latency, ms
tefe18ab9 = Gecikme, ms

# Down, kbit/s
t08c4ae33 = İndirme, kbit/s

# Up, kbit/s
td55cfe9a = Yükleme, kbit/s

# Loss, %
tea4af439 = Kayıp, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Yalnızca bu host'lar, örneğin *.example.com; boşsa tümü

# Not kept for next time: {e}
tdce4d67c = Sonraki sefer için saklanmadı: { $e }

# Slow network
td51ddfce = Yavaş ağ

# Off: traffic goes as fast as it can
tf0f34db0 = Kapalı: trafik olabildiğince hızlı akar

# Custom
t9525b78e = Özel

# Only these hosts
tfe16992b = Yalnızca bu host'lar

# Apply
tca77bbdb = Uygula

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Sonraki istekten itibaren geçerlidir. Gecikme her gidiş-dönüşe eklenir; hız, yavaşlatılan tüm akışların paylaştığı tek bir bağlantıdır; kaybolan bir parça, TCP'de olduğu gibi yeniden gönderilmeyi bekler. Diğer host'lar yavaşlatılmaz.

# Header line {line} has no ":"
t891b076d = Başlık satırı { $line }: ":" yok

# Header line {line} has no name
te3247b96 = Başlık satırı { $line }: ad yok

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
        [one] metin olmayan { $n } bayt: geldiği gibi iletilir.
       *[other] metin olmayan { $n } bayt: geldiği gibi iletilir.
    }

# Nothing is paused
t8cf68b4c = Duraklatılmış bir şey yok

# "{status}" is not a status
t40a529af = "{ $status }" bir durum kodu değil

# REQUEST
t1f3f6dd2 = İSTEK

# RESPONSE
tc85f77de = YANIT

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Duraklatılmış bir şey yok. Trafiği burada durdurmak için Kurallar ekranında bir Duraklat kuralı ekleyin.

# for {method} {url}
t093a86f6 = { $method } { $url } için

# The request, before the server gets it
tc840fed6 = İstek, sunucu almadan önce

# The response, before the app gets it
t6683d72a = Yanıt, uygulama almadan önce

# Send as it came
tb593e730 = Geldiği gibi gönder

# Drop
t108524ac = At

# Paused
tde281feb = Duraklatılanlar

# Nothing is waiting.
tea613769 = Bekleyen bir şey yok.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
        [one] { $n } öğe iletilmeyi bekliyor.
       *[other] { $n } öğe iletilmeyi bekliyor.
    }

# Send all as they came
tdce25640 = Tümünü geldiği gibi gönder

# The list was not saved: {e}
tfceba380 = Liste kaydedilmedi: { $e }

# {count} service: {names} | {count} services: {names}
tc1b8764c =
    { $n ->
        [one] { $count } servis: { $names }
       *[other] { $count } servis: { $names }
    }

# Add
t9dc3aa14 = Ekle

# https://api.example.com or http://localhost:50051
t14f9744c = https://api.example.com ya da http://localhost:50051

# Reading…
t6aad8485 = Okunuyor…

# {count} file | {count} files
tde674bd8 =
    { $n ->
        [one] { $count } dosya
       *[other] { $count } dosya
    }

# {count} message | {count} messages
t8a1cd2f6 =
    { $n ->
        [one] { $count } mesaj
       *[other] { $count } mesaj
    }

# {count} service | {count} services
t51f71902 =
    { $n ->
        [one] { $count } servis
       *[other] { $count } servis
    }

# and {count} more problem | and {count} more problems
t591ce5ea =
    { $n ->
        [one] ve { $count } sorun daha
       *[other] ve { $count } sorun daha
    }

# Remove
t21a5901d = Kaldır

# Protobuf schemas
ta4401d1a = Protobuf şemaları

# Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
te5f4f07c = .proto dosyalarının olduğu klasörler ya da protoc -o ile yapılmış descriptor set'ler; Protobuf ve gRPC gövdelerini alan adlarıyla göstermek için. Bir gRPC çağrısı mesajlarını yolundan, diğer Protobuf türündeki messageType'tan bulur. Değiştiklerinde yeniden okunur.

# Add a folder…
tb5930e43 = Klasör ekle…

# Add files…
ted08dbe5 = Dosya ekle…

# Read again
t2f0dcc5d = Yeniden oku

# Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.
tf6948a5a = Ya da gRPC reflection sunan bir sunucudan dosyalarını isteyin. https:// tracewing üzerinden gider; http:// TLS'siz gRPC'dir, doğrudan sunucuya gider.

# Asking…
tf9cf22fe = Soruluyor…

# Fetch
t533a3183 = Getir

# Find a message
te6f78626 = Mesaj bul

# Read as {message}
t87b6a380 = { $message } olarak oku

# Read as…
t1b90af95 = Şöyle oku…

# Requests to {path} on {host} are read as the message chosen.
t8826d766 = { $host } üzerindeki { $path } adresine giden istekler seçilen mesaj olarak okunur.

# Answers from {path} on {host} are read as the message chosen.
tbbe012da = { $host } üzerindeki { $path } adresinden gelen yanıtlar seçilen mesaj olarak okunur.

# Automatic (gRPC path or messageType)
t134c9a06 = Otomatik (gRPC yolu ya da messageType)

# and {count} more: type to narrow them | and {count} more: type to narrow them
t70ff68f9 =
    { $n ->
        [one] ve { $count } tane daha: daraltmak için yazın
       *[other] ve { $count } tane daha: daraltmak için yazın
    }

# Write the server's address first
t4f68f91a = Önce sunucunun adresini yazın

# "{address}" is not a server's address
t8c4d03a6 = "{ $address }" bir sunucu adresi değil

# The server has no services to describe
tfa5b7ad8 = Sunucunun tanımlanacak servisi yok

# The server sent no files for its services
tdca19bdc = Sunucu servisleri için dosya göndermedi

# The server answered {status}: it may not offer gRPC reflection
t345fdb40 = Sunucu { $status } yanıtını verdi: gRPC reflection sunmuyor olabilir

# The server answered {code}: it may not offer gRPC reflection. {message}
tb629329e = Sunucu { $code } yanıtını verdi: gRPC reflection sunmuyor olabilir. { $message }

# The server answered nothing: it may not offer gRPC reflection
t6bbb6207 = Sunucu hiçbir şey göndermedi: gRPC reflection sunmuyor olabilir

# The password is kept in the system's keychain.
t3a48373c = Parola sistemin anahtar zincirinde saklanıyor.

# The password is kept in relay.toml, which only you can read.
tf84cb18b = Parola, yalnızca sizin okuyabildiğiniz relay.toml içinde saklanıyor.

# The password is kept in relay.toml, which only you can read: the keychain cannot be used ({why}).
t5a08b6d0 = Parola, yalnızca sizin okuyabildiğiniz relay.toml içinde saklanıyor: anahtar zinciri kullanılamıyor ({ $why }).

# New connections go through the {kind} proxy {address}.
tb5f1ef96 = Yeni bağlantılar { $kind } proxy { $address } üzerinden gider.

# Connections go straight to servers.
t4a64a08f = Bağlantılar sunuculara doğrudan gider.

# relay.toml cannot be used, so connections go straight: {problem}
tbc6a2da3 = relay.toml kullanılamıyor, bu yüzden bağlantılar doğrudan gidiyor: { $problem }

# http://proxy.example.com:8080 or socks5://proxy.example.com:1080
tb945a053 = http://proxy.example.com:8080 veya socks5://proxy.example.com:1080

# Go through another proxy
tc67dfdb1 = Başka bir proxy üzerinden git

# Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080; the password is then kept in the system's keychain, not shown again. Empty, tracewing goes straight to servers.
t5c30a22f = Bir şirket ağı dışarıya yalnızca kendi proxy'si üzerinden çıkmaya izin veriyorsa, tracewing sunuculara o proxy üzerinden gider. Uygulamalar trafiklerini yine tracewing'e gönderir. Kullanıcı adı ve parola adrese yazılır, örneğin http://ad:parola@proxy:8080; parola sonra sistemin anahtar zincirinde saklanır ve yeniden gösterilmez. Boşsa tracewing sunuculara doğrudan gider.

# Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host.
tcbc7d298 = Doğrudan ulaşılacak sunucular, NO_PROXY'deki gibi: example.com kendisi ve altındaki her şey, * her sunucu.

# Sent by the server, not asked for
td717c770 = Sunucunun kendiliğinden gönderdiği, istenmiş değil

# any
t2c29f04d = herhangi

# Failed
t5579c3d4 = Başarısız

# The server answered 200: a call that fails says so in the body.
ta74e5f73 = Sunucu 200 yanıtladı: başarısız bir çağrı bunu gövdede söyler.

# {count} item | {count} items
t534e422e = { $count } öğe

# {count} field | {count} fields
t0039810a = { $count } alan

# and {count} more
tbdb1ff6a = ve { $count } tane daha

# {count} byte that does not read as one | {count} bytes that do not read as one
ta7ee514c = { $count } bayt, bir işlem olarak okunmuyor

# {program}, {count} bytes
t1ce6ad3b = { $program }, { $count } bayt

# succeeded
t6b166ff2 = başarılı

# failed: it was mined, and what it asked for did not happen
t46ea918c = başarısız: bloğa girdi, ama istediği şey gerçekleşmedi

# none: it makes a contract
t188b58f6 = yok: bir sözleşme oluşturur

# {count} byte, which the signature does not say how to read | {count} bytes, which the signature does not say how to read
t36ffce30 = { $count } bayt, imza bunların nasıl okunacağını söylemiyor

# {count} place | {count} places
t0965779e = { $count } yer

# {count} blob | {count} blobs
t95369f92 = { $count } blob

# 0, the kind from before EIP-2718
tc2fde48e = 0, EIP-2718 öncesinin türü

# The bodies were not kept, so what the call sent cannot be shown.
teaf455ca = Gövdeler saklanmadı, bu yüzden çağrının ne gönderdiği gösterilemiyor.

# and {count} more calls
tf0ac7103 = ve { $count } çağrı daha

# {count} function | {count} functions
t9ffc3c7c = { $count } fonksiyon

# {count} event | {count} events
t4dd03992 = { $count } olay

# {count} error | {count} errors
t4fba4c4e = { $count } hata

# Contract ABIs
t08fb00f2 = Sözleşme ABI'leri

# Folders or files of ABI JSON, as solc, Hardhat and Foundry write it, to name the functions, events and errors in a JSON-RPC call instead of showing four bytes. They are read again when they change. Nothing is sent anywhere: a signature is worked out from the ABI on this machine.
tfadffbd2 = solc, Hardhat ve Foundry'nin yazdığı biçimde ABI JSON klasörleri veya dosyaları; bir JSON-RPC çağrısındaki fonksiyonları, olayları ve hataları dört bayt yerine adıyla göstermek için. Değiştiklerinde yeniden okunur. Hiçbir yere bir şey gönderilmez: imza bu makinede ABI'den hesaplanır.

# Block
tbd074002 = Engelle

# Answer from a file
td8edab50 = Dosyadan yanıtla

# Send to another host
te3cd85d7 = Başka bir host'a gönder

# Set a request header
tfad4aaaa = İstek başlığı ayarla

# Set a response header
tcbbc20d0 = Yanıt başlığı ayarla

# Remove a request header
t5228dd84 = İstek başlığını kaldır

# Remove a response header
teeb5735a = Yanıt başlığını kaldır

# Set a query parameter
t73d9419b = Sorgu parametresi ayarla

# Remove a query parameter
t12633da9 = Sorgu parametresini kaldır

# Change the request body
t41101201 = İstek gövdesini değiştir

# Change the response body
tdd259b73 = Yanıt gövdesini değiştir

# Change the status
t4bbdfba8 = Durumu değiştir

# Pause the request
t40ecc39b = İsteği duraklat

# Pause the response
t18a34799 = Yanıtı duraklat

# Status to answer with, like 403
t8f3ebb6c = Yanıt durum kodu, örneğin 403

# Full path of a file or folder
t9fb9ae4c = Bir dosyanın ya da klasörün tam yolu

# Where to, like http://localhost:3000
tdfedd004 = Nereye, örneğin http://localhost:3000

# Header's name
t80793085 = Başlığın adı

# Its value
t43968778 = Değeri

# Parameter's name
t313c8e35 = Parametrenin adı

# Text to find, or empty for the whole body
t8255fc87 = Aranacak metin, ya da tüm gövde için boş

# What to put in its place
t03671fd5 = Yerine konacak

# Status the client gets, like 500
t5e4d3088 = İstemcinin alacağı durum, örneğin 500

# "{first}" is not a status
td43128d3 = "{ $first }" bir durum kodu değil

# Block with {status}
t1727e984 = Engelle: { $status }

# Answer from {path}
taefc08fa = Dosyadan yanıtla: { $path }

# Send to {to}
t5e256873 = Şuraya gönder: { $to }

# Request: {header}: {value}
taa230766 = İstek: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Yanıt: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = İstek: { $header } yok

# Response: no {header}
t23960a2e = Yanıt: { $header } yok

# Query: {name}={value}
t0ee6c1c0 = Sorgu: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Sorgu: { $name } yok

# Status {status}
tad048e53 = Durum { $status }

# Request body: {replace}
te4856c36 = İstek gövdesi: { $replace }

# Response body: {replace}
t8600dc42 = Yanıt gövdesi: { $replace }

# Request body: {find} → {replace}
taec471a9 = İstek gövdesi: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Yanıt gövdesi: { $find } → { $replace }

# Name
t0fe07306 = Ad

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, örneğin api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Metot, ya da tümü için boş

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
        [one] { $n } kural şuraya yazıldı: { $path }
       *[other] { $n } kural şuraya yazıldı: { $path }
    }

# The rules were not written: {e}
te7ac257f = Kurallar yazılamadı: { $e }

# Import
t573e0af4 = İçe aktar

# {path} could not be read: {e}
t32a3ce94 = { $path } okunamadı: { $e }

# Nothing was imported: {e}
tb9f7257a = Hiçbir şey içe aktarılmadı: { $e }

# Edit
tc2c76cb1 = Düzenle

# Delete
t5797ea6a = Sil

# Change this rule
t9160f393 = Bu kuralı değiştir

# A new rule
t55a722a0 = Yeni kural

# It waits under Paused until you send it on, changed or not.
tca566a57 = Siz iletene kadar, değiştirilmiş ya da değiştirilmemiş, Duraklatılanlar altında bekler.

# Save
t4d2d5d68 = Kaydet

# Cancel
t35afca3b = İptal

# Rules
t428b2d80 = Kurallar

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Kurallar sonraki istekten itibaren, kayıt duraklatılmışken de geçerlidir. Yukarıdan aşağıya: ilk engelleme, dosya ya da host kazanır, uyan diğer her kural sırayla uygulanır. Gövdeyi ya da durumu değiştiren bir kural, sunucunun gönderdiğini değiştirir.

# Import rules…
t5edc205d = Kuralları içe aktar…

# Export rules…
t6e09ca20 = Kuralları dışa aktar…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
        [one] { $n } kural eklendi
       *[other] { $n } kural eklendi
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
        [one] { $n } tanesi zaten var
       *[other] { $n } tanesi zaten var
    }

# left out: {why}
ta81e95e2 = dışarıda bırakıldı: { $why }

# Copy all
t6aef5c37 = Tümünü kopyala

# Too large to copy all
t2fe8ff9f = Tümünü kopyalamak için çok büyük

# No flows
t6dc463eb = Akış yok

# Name it to keep it
t52c1a402 = Saklamak için ad verin

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · başka bir sürüm tarafından yazılmış, bu yüzden açılamıyor

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
        [one] { $count } akış · { $size }
       *[other] { $count } akış · { $size }
    }

# Recording now
t3cb3e6fc = Şu anda kaydediliyor

# Export HAR
t4d6458cc = HAR dışa aktar

# Delete for good?
ta7737f13 = Kalıcı olarak silinsin mi?

# Sessions
tb69a8aec = Oturumlar

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Adı olan bir oturum, siz silene kadar saklanır. Diğerlerinden en yeni { $kept } tanesi kalır.

# Working…
td5507854 = Çalışıyor…

# Open a session file
tae8622fc = Oturum dosyası aç

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, Fiddler .saz ya da Charles .chlsj

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = Dışa aktarılan HAR dosyaları başlıklarda, URL'lerde ve gövdelerde çerezleri, token'ları, anahtarları ve parolaları dışarıda bırakır

# Other devices
t9e7d8362 = Diğer cihazlar

# Let other devices use the proxy
t13e626b3 = Diğer cihazların proxy'yi kullanmasına izin ver

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = Böylece ağınızdaki telefonlar ve diğer bilgisayarlar trafiklerini buraya gönderebilir. O ağdaki herkes de gönderebilir: güvenmediğiniz bir ağda bunu kapalı tutun.

# Servers
tcd0cf113 = Sunucular

# Trust what this computer trusts
tfea2d91b = Bu bilgisayarın güvendiklerine güven

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = Bu bilgisayarda yüklü bir sertifika yetkilisinden (örneğin şirketin kendi CA'sından) sertifika almış sunuculara, bu bilgisayarın tarayıcıları nasıl ulaşıyorsa öyle ulaşılır. Kapalıyken yalnızca genel yetkililere güvenilir. tracewing yeniden başladığında etkili olur.

# AI tools
t294d7ea4 = Yapay zekâ araçları

# Let AI tools see tokens and cookies
te1d26d3c = Yapay zekâ araçlarının token'ları ve çerezleri görmesine izin ver

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = Yapay zekâ araçları kaydedilen trafiği tracewing'in MCP sunucusu üzerinden okur. Kapalıyken Authorization, Cookie ve Set-Cookie değerleri onlara verilmez. Hemen uygulanır.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Uygulamaları HTTP/3'ten uzak tut

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = HTTP/3, hiçbir proxy'nin taşımadığı UDP üzerinden gider, bu yüzden onu kullanan bir uygulama tracewing'i atlar. Bu ayar, sunucuların HTTP/3 tekliflerini yanıtlarından çıkarır ve uygulamalar HTTP/2'de kalır. Akışlar teklifleri yine sunucuların gönderdiği gibi gösterir.

# Size of a session
ta20fa102 = Oturum boyutu

# Past this, the oldest flows are dropped.
td2914f46 = Bu aşılınca en eski akışlar atılır.

# Room to leave on the disk
t8674a3b3 = Diskte bırakılacak boş alan

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Boş alan bundan azsa akışlar yine listelenir ama gövdeler saklanmaz.

# Earlier sessions to keep
td356194a = Saklanacak önceki oturumlar

# Older ones are deleted when a new session starts.
t66556513 = Yeni bir oturum başladığında daha eskiler silinir.

# Bodies
tdb4d0409 = Gövdeler

# Most kept of one body
t777508dd = Bir gövdeden en fazla saklanan

# It still reaches the client whole.
t386fee28 = İstemciye yine de bütün olarak ulaşır.

# Most unpacked for viewing
tc3fe2efd = Görüntülemek için en fazla açılan

# Stops a small download that unpacks to many GB.
tf9c49515 = Açıldığında GB'larca yer tutan küçük bir indirmeyi durdurur.

# Small body size
ta8996af1 = Küçük gövde boyutu

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Daha küçük gövdeler bellekte bekler ve bir dosyayı paylaşır. Daha büyükleri geldikçe kendi dosyalarını alır.

# Compress plain text on disk
tee503c2e = Düz metni diskte sıkıştır

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Yaklaşık yarı yer kaplar; trafik buna yetişemeyecek kadar hızlıysa atlanır.

# Compression level
td386c9ef = Sıkıştırma düzeyi

# 1 to 19. Higher is smaller and slower.
td42a987d = 1 ile 19 arası. Yüksek olan daha küçük ve daha yavaştır.

# Search
tc646a2c9 = Arama

# Search URLs
t0c3e0b3d = URL'lerde ara

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Yazarken URL'nin herhangi bir parçasını bulur. Az maliyetlidir.

# Search bodies
t8eec0293 = Gövdelerde ara

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Gövdelerdeki kelimeleri bulur. Gövdelerin kendisi kadar disk kaplayabilir.

# Searched part of each body
tab293b21 = Her gövdenin aranan kısmı

# Words past this are not found. More takes more disk.
t0bf65f4a = Bunun ötesindeki kelimeler bulunmaz. Daha fazlası daha çok disk kaplar.

# Under heavy traffic
ta99888cb = Yoğun trafikte

# Memory for bodies waiting to be written
t7d6ff799 = Yazılmayı bekleyen gövdeler için bellek

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Bu aşılınca akışlar gövdeleri olmadan listelenir. Trafik asla bekletilmez.

# Memory for large bodies waiting to be written
t07b9de08 = Yazılmayı bekleyen büyük gövdeler için bellek

# Past this, large bodies stop being kept.
tfe61a4f3 = Bu aşılınca büyük gövdeler saklanmaz.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }" bir tam sayı değil

# {key}: must not be negative
t9c4debaa = { $key }: negatif olamaz

# After a restart
t58dcea2d = Yeniden başlatınca

# Every setting is back to its default.
t763785e7 = Tüm ayarlar varsayılanlarına döndü.

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d =
    { $n ->
        [one] { $n } değişiklik tracewing yeniden başladığında etkili olur.
       *[other] { $n } değişiklik tracewing yeniden başladığında etkili olur.
    }

# Settings
t4b058728 = Ayarlar

# Undo
t71fd4acf = Geri al

# Back to defaults
ta3162a13 = Varsayılanlara dön

# Hosts not to decrypt
tb457f0b3 = Şifresi çözülmeyecek host'lar

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = Satır başına bir tane: bir host, ya da kendisi ve altındaki her şey için *.example.com. Bunların trafiği okunmadan geçer. Bu liste hemen uygulanır.

# Authorities trusted for servers
t0a308a8a = Sunucular için güvenilen sertifika yetkilileri

# tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in {folder}. They are read when tracewing starts.
t571051f9 = tracewing, genel sertifika yetkililerinin ve bu bilgisayarın güvendiklerinin onayladığı sunuculara güvenir. Bunlara da güvenir: bir şirketin kendi yetkilisi ya da sizin kendi yetkiliniz, { $folder } içinde .pem dosyası olarak. tracewing başlarken okunurlar.

# Left out: {why}
t4a567d82 = Atlandı: { $why }

# None yet.
t786bc275 = Henüz yok.

# The folder has changed since tracewing started: restart it to use what it holds now.
t65f2d800 = Klasör, tracewing başladığından beri değişti: şimdiki içeriği kullanmak için yeniden başlatın.

# Open the folder
tc7d7e212 = Klasörü aç

# Read it again
td72efd5e = Yeniden oku

# Language
t9a73db9b = Dil

# System ({system_name})
tb5c7a1d4 = Sistem ({ $system_name })

# Applies at once.
tefb7a0f9 = Hemen uygulanır.

# Your browser
tb5a48dd2 = Tarayıcınız

# Its private key is in the system keychain.
t62a47808 = Özel anahtarı sistem anahtar zincirinde.

# Its private key is in ca.key, which only you can read.
td9c6165d = Özel anahtarı, yalnızca sizin okuyabildiğiniz ca.key dosyasında.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = Özel anahtarı, yalnızca sizin okuyabildiğiniz ca.key dosyasında, çünkü sistem anahtar zinciri onu alamıyor: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Sayfayı açın. Tarayıcı sertifika hakkında uyarırsa ya da sayfayı bulamazsa, yukarıdaki adımlardan biri o tarayıcı için yapılmamıştır.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name }, tracewing üzerinden geçiyor ve ona güveniyor.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name }, tracewing üzerinden geçiyor ama sayfayı HTTPS olmadan açtı. https:// ile yeniden açın.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = Bir telefon ya da başka bir bilgisayar da trafiğini buraya gönderebilir. Ayarlar'da “Diğer cihazların proxy'yi kullanmasına izin ver”i açın.

# this computer's address, port {port}
t2188b3f4 = bu bilgisayarın adresi, port { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = Cihazda Wi-Fi proxy'sini { $addr } olarak ayarlayın. Sonra orada http://{ $host } adresini açın, sertifikayı alın ve ona güvenin. Cihazın akışları adresini gösterir ve device: ile filtrelenebilir. Cihaza, akışlarından birinden, Bağlantı bölümünde ad verin.

# Looking for it…
t50ee7452 = Aranıyor…

# Installing. The system may ask you to confirm.
ta41c38ec = Yükleniyor. Sistem onaylamanızı isteyebilir.

# Removing. The system may ask you to confirm.
t62b22726 = Kaldırılıyor. Sistem onay isteyebilir.

# This computer trusts it.
t27d6bacc = Bu bilgisayar ona güveniyor.

# Installed in: {places}.
tb5bcc244 = Yüklendiği yerler: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Komut satırı araçları ona güveniyor, ama tarayıcıların kendi depoları var. Sertifika onlara eklenene kadar, tracewing üzerinden HTTPS tarayıcıda başarısız olur.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing HTTPS'i yalnızca bu bilgisayar için üretilmiş bir sertifikayla okur. Ona güvenilene kadar, tracewing üzerinden HTTPS başarısız olur.

# Trust the certificate
t26618699 = Sertifikaya güven

# Install
tb50cbd40 = Yükle

# Also for command-line tools
t0db8faea = Komut satırı araçları için de

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Sertifika bu bilgisayardaki tüm depolardan kaldırılsın mı? Yeniden yüklenene kadar tracewing üzerinden HTTPS başarısız olur. tracewing sertifikayı daha sonra yeniden yüklemek için saklar.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = Linux'ta tarayıcılara sertifikaya güvenmelerini söylemek için certutil gerekir. Onu yükleyin, sonra burada Yükle'ye basın:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = Linux'ta tarayıcılara sertifikaya güvenmelerini söylemek için certutil gerekir. Dağıtımınızın NSS araçlarını yükleyin, sonra burada Yükle'ye basın.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = Bu masaüstünde tracewing'in değiştirebileceği bir sistem proxy ayarı yok. Aşağıda Tarayıcılar'dan bir tarayıcıyı ayrı açın ya da bir terminal açın.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } masaüstünde tracewing'in değiştirebileceği bir sistem proxy ayarı yok. Aşağıda Tarayıcılar'dan bir tarayıcıyı ayrı açın ya da bir terminal açın.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Sistem ayarını kullanan uygulamalar artık { $addr } üzerinden geçiyor. tracewing kapandığında ya da çökerse eski hâline döner.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Bu bilgisayarın trafiğini, sistem ayarını kullanan uygulamalar için tracewing üzerinden gönderir. Ya da bir uygulamayı kendiniz { $addr } adresine yönlendirin.

# Turn on the system proxy
t52b88441 = Sistem proxy'sini aç

# Turn off
tcd03e04b = Kapat

# Turn on
t65a0c5a7 = Aç

# Check it in your browser
tbed6e83a = Tarayıcınızda denetleyin

# Open the test page
te4e36f65 = Test sayfasını aç

# Set up
t43c06abc = Kurulum

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Bu bilgisayarın HTTPS trafiğini görmek için üç adım.

# See traffic
t980eee79 = Trafiği gör

# the browsers' own stores
tddf6836c = tarayıcıların kendi depoları

# Trusts the certificate.
tc5bba858 = Sertifikaya güveniyor.

# Does not trust the certificate yet.
tf2a0b8c5 = Sertifikaya henüz güvenmiyor.

# Looking…
t284579ba = Aranıyor…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Henüz profili yok. Bir kez açın, sonra yeniden arayın.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 =
    { $n ->
        [one] { $n } profilinden { $holding } tanesi sertifikaya güveniyor.
       *[other] { $n } profilinden { $holding } tanesi sertifikaya güveniyor.
    }

# Has not opened the test page yet.
t67f9cb49 = Test sayfasını henüz açmadı.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Test sayfasını tracewing üzerinden, HTTPS ile açtı.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Test sayfasını tracewing üzerinden açtı, ama HTTPS olmadan.

# Browsers
td580efd4 = Tarayıcılar

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Her biri neye güveneceğine ve trafiğini nereye göndereceğine kendisi karar verir. Denetle, test sayfasını onda açar. Ayrı aç, onu tracewing üzerinden geçen kendi profiliyle başlatır; sistem proxy'sine dokunmaz.

# Look again
t98416222 = Yeniden bak

# No browser found on this computer.
t0e46bb34 = Bu bilgisayarda tarayıcı bulunamadı.

# Check
t592c8de7 = Denetle

# Open on its own
t0da7783c = Ayrı aç

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = Bu bilgisayarda UDP port 443 engelli, bu yüzden uygulamalar HTTP/3'ten HTTP/2'ye geri düşer ve tracewing üzerinden geçer. Engel, tracewing kapandığında ya da çökerse kalkar.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Bazı uygulamalar UDP üzerinden HTTP/3 konuşur ve proxy'yi atlar. Bu bilgisayarda UDP port 443'ü engellemek, onları tracewing üzerinden HTTP/2'ye geri düşürür. Yönetici parolası gerektirir ve tracewing kapandığında sona erer. Bu sürede UDP port 443'ü kullanan bir VPN çalışmaz.

# Apps on HTTP/3
t2cfe5270 = HTTP/3 kullanan uygulamalar

# Stop blocking
t318358d8 = Engellemeyi durdur

# Block HTTP/3
t88231e40 = HTTP/3'ü engelle

# Or one app at a time
t91c63f1c = Ya da tek tek uygulamalar

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Sistem proxy'si ve diğer tüm uygulamalar olduğu gibi kalırken tracewing üzerinden geçen bir terminal. Bir tarayıcı için Tarayıcılar'daki Ayrı aç, aynısını kendi profiliyle yapar.

# Open a terminal
t4b629c72 = Terminal aç

# iPhone and iPad
ta550cc97 = iPhone ve iPad

# Flutter apps
ta04ee397 = Flutter uygulamaları

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. Ayarlar, Wi-Fi'de ağı seçin, sonra Vekil Sunucuyu Yapılandır, Elle: sunucu { $server }, port { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Kodu kamerayla tarayın ya da Safari'de { $page } adresini açın ve sertifikayı alın. İndirmeye izin verin.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. Ayarlar, Genel, VPN ve Aygıt Yönetimi'nde tracewing CA profilini seçin ve yükleyin.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. Ayarlar, Genel, Hakkında, Sertifika Güven Ayarları'nda tracewing CA için tam güveni açın.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Safari'de https://{ $host } adresini açın. Çalıştığını söylüyorsa, sistem ayarlarını kullanan uygulamalar tracewing üzerinden geçer.

# When you are done
t62466f08 = İşiniz bitince

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. Ayarlar, Wi-Fi'de ağı seçin, sonra Proxy'yi Yapılandır: Kapalı. Açık bırakılırsa tracewing kapandığında cihazın bu ağda interneti olmaz.

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. Ayarlar, Genel, VPN ve Aygıt Yönetimi'nde tracewing CA'yı seçip profili kaldırın. Tam güveni de onunla gider.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. Ayarlar, Wi-Fi'de ağı düzenleyin ve Gelişmiş seçenekler altında proxy'yi Manuel olarak ayarlayın: ana makine { $host }, port { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Kodu tarayın ya da tarayıcıda { $page } adresini açın ve sertifikayı alın.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. Ayarlar, Güvenlik, Şifreleme ve kimlik bilgileri bölümünde (adlar üreticiye göre değişir) Sertifika yükle, ardından CA sertifikası seçeneğine dokunun ve tracewing.crt dosyasını seçin.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Android 7'den beri uygulamalar, kullanıcının eklediği sertifikalara yalnızca bunu belirtirlerse güvenir. Kendi uygulamanızın debug derlemesi için res/xml/network_security_config.xml dosyasını ekleyin:

# and point to it from AndroidManifest.xml:
t02ad844b = ve AndroidManifest.xml içinden ona başvurun:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Diğer uygulamalar sertifikayı reddetmeye devam eder: trafikleri başarısız akışlar olarak görünür.

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. Ayarlar, Wi-Fi'de ağı düzenleyin ve proxy'yi yeniden Yok yapın. Açık bırakılırsa tracewing kapandığında cihazın bu ağda interneti olmaz.

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. Ayarlar, Güvenlik, Şifreleme ve kimlik bilgileri, Kullanıcı kimlik bilgileri'nde tracewing'i seçip kaldırın. tracewing.crt'yi İndirilenler'den de silin.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = Flutter'ın http ve Dio paketlerinin kullandığı Dart HttpClient, telefonun proxy ayarını izlemez ve Android'de kullanıcının eklediği sertifikalara güvenmez. Bir debug derlemesinde onu buraya yönlendirin:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback her sertifikayı geçirir, bu yüzden yalnızca debug derlemeleri içindir.

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = Yayından önce TracewingProxy'yi ve main() içindeki satırını çıkarın; debug derlemeleri dışında hiçbir şey yapmasalar da. Telefonda hiçbir şey değişmedi.

# Open settings
t1336b4d6 = Ayarları aç

# Phones and other devices
t2e3c591c = Telefonlar ve diğer cihazlar

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Bu ağdaki bir telefon, tablet ya da başka bir bilgisayar { $proxy } adresinde tracewing üzerinden geçebilir. Kod, sertifikanın bulunduğu sayfayı açar. Cihazın akışları adresini gösterir, device: ile filtrelenebilir ve cihaza akışlarından birinden, Bağlantı bölümünde ad verilebilir.

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = Diğer cihazlar bu bilgisayarın adreslerinden birinde tracewing'e ulaşamıyor: { $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f =
    { $n ->
        [one] { $n } cihaz tracewing üzerinden geçti ({ $names }). İşiniz bitince, aşağıdaki gibi Wi-Fi proxy'sini kapatın: açık bırakılırsa tracewing kapandığında bu ağda interneti olmaz.
       *[other] { $n } cihaz tracewing üzerinden geçti ({ $names }). İşiniz bitince, aşağıdaki gibi Wi-Fi proxy'lerini kapatın: açık bırakılırsa tracewing kapandığında bu ağda internetleri olmaz.
    }

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Kuruldu. Sertifika, cihaz yeniden başlayana kadar sistem sertifikaları arasında.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Kuruldu. Sertifika İndirilenler'de: az önce açılan Ayarlar'da sırasıyla Şifreleme ve kimlik bilgileri, Sertifika yükle, CA sertifikası seçeneklerine dokunun ve tracewing.crt dosyasını seçin.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Yalnızca sistem sertifikalarına güvenen uygulamalar onu yine de reddeder.

# An emulator.
t14b6b86d = Bir emülatör.

# On USB.
t67dafc14 = USB üzerinden.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Cihazdaki USB hata ayıklama isteğini kabul edin, sonra yeniden bakın.

# Still starting. Look again in a moment.
t13f68b64 = Hâlâ başlıyor. Birazdan yeniden bakın.

# adb says it is {state}.
t2b7d71da = adb'ye göre durumu: { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Bir Android emülatöründe ya da USB hata ayıklaması açık, USB ile bağlı bir telefonda proxy'yi ayarlar ve sertifikayı yükler. Trafik ADB üzerinden gelir, bu yüzden ikisinin de bu ağda olması gerekmez.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = adb bulunamadı. Android Studio'yu ya da Android platform araçlarını yükleyin, sonra yeniden bakın.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Bağlı cihaz yok. Bir emülatör başlatın ya da USB hata ayıklaması açık bir telefon takın, sonra yeniden bakın.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Bir cihazı, trafiğini buraya gönderecek şekilde ayarlayın. Geri al ya da tracewing'i kapatmak proxy'yi yeniden kaldırır.

# Android emulators and USB devices
t379e433c = Android emülatörleri ve USB cihazları

# Find devices
t54d4af39 = Cihazları bul

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = Sistem proxy'si ve HTTP/3 engeli, tracewing kapandığında ya da çöktüğünde eski hâline döner; USB ile kurulan bir Android cihaz da öyle. Sertifika, 1. adımdaki Kaldır onu çıkarana kadar güvenilir kalır. Elle kurulan bir telefon, Wi-Fi proxy'sini ve sertifikasını orada kaldırılana kadar tutar: her cihazın kılavuzu nasıl yapılacağını söyler. Diğer cihazlar, bu Ayarlar'da kapatılana kadar bu bilgisayarın proxy'sine ulaşabilir.

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code ve diğer yapay zekâ araçları, tracewing'in kaydettiği akışları MCP sunucusu üzerinden listeleyip okuyabilir. Ayarlar izin vermedikçe token'lar ve çerezler dışarıda kalır. Claude Code'da:

# Elsewhere, in the tool's MCP settings:
tec38833c = Başka araçlarda, aracın MCP ayarlarında:

# Flows
t4db77db2 = Akışlar

# Choose the flow above or below
tecb3adec = Üstteki ya da alttaki akışı seç

# The first or the last flow
t7e278a00 = İlk ya da son akış

# A page up or down
t23b1814b = Bir sayfa yukarı ya da aşağı

# Filter
tf4a9c097 = Filtrele

# Pause or resume recording
t6e21519a = Kaydı duraklat ya da sürdür

# Clear the list
tf2a55d1f = Listeyi temizle

# Compare the flow clicked with the chosen one
te42cd067 = Tıklanan akışı seçili olanla karşılaştır

# The chosen flow
te0a729de = Seçili akış

# Its request
tb3970784 = İsteği

# Its response
t221fc168 = Yanıtı

# Send it again
tef7d1944 = Yeniden gönder

# Edit it and send it
t6bc99e5c = Düzenle ve gönder

# Copy it as cURL
tc99a11a3 = cURL olarak kopyala

# The part before or after the one shown
tdf4221d9 = Gösterilenden önceki veya sonraki bölüm

# Save the body shown
t20d9b7c0 = Gösterilen gövdeyi kaydet

# Select all of the part shown
tf9168168 = Gösterilen bölümün tümünü seç

# Copy the selected text
t00b090d3 = Seçili metni kopyala

# Send it on, as edited
t8c35fa5a = Düzenlendiği gibi gönder

# Send it on as it was
t16f7abb5 = Olduğu gibi gönder

# Drop it
t4e7f48b9 = At

# The window
t2af18488 = Pencere

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Kurulum, Trafik, Kurallar, Duraklatılanlar, Oluştur, Oturumlar, Ayarlar

# Show or hide the sidebar
t67adca04 = Kenar çubuğunu göster ya da gizle

# Show or hide the inspector
ta5a7b9e9 = İnceleyiciyi göster ya da gizle

# The inspector beside the list or below it
t51e214f7 = İnceleyici listenin yanında ya da altında

# The next or the previous field
ta4b4bd5e = Sonraki veya önceki alan

# These shortcuts
tbdf3b62b = Bu kısayollar

# Close them
tf3a4b1ef = Kısayolları kapat

# Keyboard shortcuts
t31eaddb7 = Klavye kısayolları

# Close
tcd86acc3 = Kapat

# The certificate: {e}
t0235648c = Sertifika: { $e }

# The terminal
tf542b2f4 = Terminal

# The system proxy: {e}
tc99726ce = Sistem proxy'si: { $e }

# Blocking HTTP/3: {e}
t024ec871 = HTTP/3 engelleme: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx, ya da gövdelerde metin

# A name, as iPhone test
t19f89214 = Bir ad, örneğin iPhone test

# A name, as Phone
tfce439bf = Bir ad, örneğin Phone

# The search index: {e}
te3c971c9 = Arama dizini: { $e }

# Click the flow to compare #{id} with.
tac1ec1d2 = #{ $id } ile karşılaştırılacak akışa tıklayın.

# An earlier session: {name}
t73446a1d = Önceki bir oturum: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Salt okunur. Dizin tamamlandıkça arama daha fazlasını kapsar. Kayıt sürüyor.

# Read-only. Recording goes on.
te2f35d9e = Salt okunur. Kayıt sürüyor.

# Back to live
t317c0453 = Canlıya dön

# {shown} of {total}
tf2cc0563 = { $shown } / { $total }

# Recording
te75cf20c = Kaydediliyor

# System proxy off · {addr}
t7e813d25 = Sistem proxy'si kapalı · { $addr }

# System proxy…
t29e7dcd0 = Sistem proxy'si…

# System proxy on · {addr}
t319ad6b9 = Sistem proxy'si açık · { $addr }

# Apps
t53d91aad = Uygulamalar

# Devices
t27acf320 = Cihazlar

# All hosts
tfc7bfe37 = Tüm host'lar

# No host has this in its name
t838c1d44 = Adında bu geçen bir host yok

# Hidden
t3c2dfe59 = Gizli

# Hosts
tcf9aefb4 = Host'lar

# Newest
t324b6d0b = En yeni

# No flow fits this filter
t7eb295f4 = Bu filtreye uyan akış yok

# Nothing yet
tbd00747c = Henüz bir şey yok

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Yukarıdan sistem proxy'sini açın ya da bir uygulamayı { $addr } adresine yönlendirin. Denemek için: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Listenin yanında

# Below the list
t23658daf = Listenin altında

# Drop it on a place · Esc puts it back
t0fa632ba = Bir yere bırakın · Esc geri koyar

# Traffic
t4b589266 = Trafik

# Certificate trusted on this computer
t7b93f551 = Sertifikaya bu bilgisayarda güveniliyor

# Checking the certificate…
tbf8942c3 = Sertifika denetleniyor…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Sertifika yükleniyor; sistem onaylamanızı isteyebilir…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Sertifika kaldırılıyor; sistem onay isteyebilir…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = Sertifikaya güvenilene kadar HTTPS başarısız olur: kurulum

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml kullanılmadı: { $problem }

# A setting was not saved: {problem}
tff1a5018 = Bir ayar kaydedilmedi: { $problem }

# The proxy stopped: {failure}
t04dcff11 = Proxy durdu: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = Disk neredeyse dolu: gövdeler saklanmıyor

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
        [one] { $count } akış kaydedilmedi
       *[other] { $count } akış kaydedilmedi
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
        [one] gövdesiz { $count } akış
       *[other] gövdesiz { $count } akış
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
        [one] { $size } altında kalmak için en eski { $count } akış atıldı
       *[other] { $size } altında kalmak için en eski { $count } akış atıldı
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
        [one] { $count } akış
       *[other] { $count } akış
    }

# Export as HAR
t95b6b766 = HAR olarak dışa aktar

# Shortcuts {keys}
t4ab0ecb4 = Kısayollar { $keys }

# Columns
tea156fdc = Sütunlar
