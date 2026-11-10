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

# Size
ta6478e7c = Boyut

# Time
tdfe4e404 = Süre

# Number
t61e9eac0 = Numara

# "{method}" is not a method
tacdcf5cb = "{ $method }" bir metot değil

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = istek gövdesi bütün olarak saklanmadı, bu yüzden yeniden gönderilemez

# name: value, one a line
t4df37bea = ad: değer, satır başına bir tane

# No body
tfa4c2902 = Gövde yok

# No flow is chosen
t550b0a98 = Seçili akış yok

# This flow: {e}
t296762f8 = Bu akış: { $e }

# Request
t82454152 = İstek

# Headers
t334f5989 = Başlıklar

# Body
t428a1095 = Gövde

# Sending…
te946e7bf = Gönderiliyor…

# Send
tf28e14cf = Gönder

# New
t8b242db1 = Yeni

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … { $count } bayt daha: Trafik'teki akışta hepsi var.

# Response
t55a982fe = Yanıt

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

# Not decrypted
t8090a1ba = Şifresi çözülmedi

# open
td35ec4c9 = açık

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

# URL
t7569633e = URL

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

# Add
t9dc3aa14 = Ekle

# Cancel
t35afca3b = İptal

# Rules
t428b2d80 = Kurallar

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every header rule that fits applies.
tb44c7227 = Sonraki istekten itibaren, kayıt duraklatılmışken de geçerlidir. Yukarıdan aşağıya: ilk engelleme, dosya ya da host kazanır ve uyan her başlık kuralı uygulanır.

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

# Exported HAR files leave out tokens and cookies
td4a2f543 = Dışa aktarılan HAR dosyalarına token'lar ve çerezler dahil edilmez

# Other devices
t9e7d8362 = Diğer cihazlar

# Let other devices use the proxy
t13e626b3 = Diğer cihazların proxy'yi kullanmasına izin ver

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust. Takes effect when tracewing starts again.
t2e14229d = Böylece ağınızdaki telefonlar ve diğer bilgisayarlar trafiklerini buraya gönderebilir. O ağdaki herkes de gönderebilir: güvenmediğiniz bir ağda bunu kapalı tutun. tracewing yeniden başladığında etkili olur.

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

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them. Takes effect when tracewing starts again.
t2aa7be0d = HTTP/3, hiçbir proxy'nin taşımadığı UDP üzerinden gider, bu yüzden onu kullanan bir uygulama tracewing'i atlar. Bu ayar, sunucuların HTTP/3 tekliflerini yanıtlarından çıkarır ve uygulamalar HTTP/2'de kalır. Akışlar teklifleri yine sunucuların gönderdiği gibi gösterir. tracewing yeniden başladığında etkili olur.

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

# Saved. The limits apply the next time tracewing starts.
tc8bd2065 = Kaydedildi. Sınırlar tracewing bir sonraki başlatıldığında uygulanır.

# Settings
t4b058728 = Ayarlar

# Back to defaults
ta3162a13 = Varsayılanlara dön

# Hosts not to decrypt
tb457f0b3 = Şifresi çözülmeyecek host'lar

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies as soon as it is saved.
t7647c310 = Satır başına bir tane: bir host, ya da kendisi ve altındaki her şey için *.example.com. Bunların trafiği okunmadan geçer. Bu liste kaydedildiği anda uygulanır.

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

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings, then start tracewing again.
t9f95b7b8 = Bir telefon ya da başka bir bilgisayar da trafiğini buraya gönderebilir. Ayarlar'da “Diğer cihazların proxy'yi kullanmasına izin ver”i açın, sonra tracewing'i yeniden başlatın.

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

# Remove
t21a5901d = Kaldır

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

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = Flutter'ın http ve Dio paketlerinin kullandığı Dart HttpClient, telefonun proxy ayarını izlemez ve Android'de kullanıcının eklediği sertifikalara güvenmez. Bir debug derlemesinde onu buraya yönlendirin:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback her sertifikayı geçirir, bu yüzden yalnızca debug derlemeleri içindir.

# Open settings
t1336b4d6 = Ayarları aç

# Phones and other devices
t2e3c591c = Telefonlar ve diğer cihazlar

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Bu ağdaki bir telefon, tablet ya da başka bir bilgisayar { $proxy } adresinde tracewing üzerinden geçebilir. Kod, sertifikanın bulunduğu sayfayı açar. Cihazın akışları adresini gösterir, device: ile filtrelenebilir ve cihaza akışlarından birinden, Bağlantı bölümünde ad verilebilir.

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

# Undo
t71fd4acf = Geri al

# Android emulators and USB devices
t379e433c = Android emülatörleri ve USB cihazları

# Find devices
t54d4af39 = Cihazları bul

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

# Select all of the part shown
tf9168168 = Gösterilen bölümün tümünü seç

# Copy the selected text
t00b090d3 = Seçili metni kopyala

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

# The search index: {e}
te3c971c9 = Arama dizini: { $e }

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

# Hosts
tcf9aefb4 = Host'lar

# All hosts
tfc7bfe37 = Tüm host'lar

# Hidden
t3c2dfe59 = Gizli

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
