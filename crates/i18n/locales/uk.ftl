# tracewing in Ukrainian. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 =
    { $n ->
        [one] … { $count } рядок { $closer }
        [few] … { $count } рядки { $closer }
        [many] … { $count } рядків { $closer }
       *[other] … { $count } рядка { $closer }
    }

# … {size} more
ta8dc6eab = … ще { $size }

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc =
    { $n ->
        [one] { $size } { $format } · { $count } рядок JSON
        [few] { $size } { $format } · { $count } рядки JSON
        [many] { $size } { $format } · { $count } рядків JSON
       *[other] { $size } { $format } · { $count } рядка JSON
    }

# The first {count} rows are shown.
t16aa1c6d = Показано лише перші { $count } рядків.

# Unfold all
t3849d44e = Розгорнути все

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } рядок
        [few] { $size } · { $count } рядки
        [many] { $size } · { $count } рядків
       *[other] { $size } · { $count } рядка
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · читання { $percent }%

# {call} could not be put in the body as calldata. Write it as hex, or let tracewing write the body.
t1d03e346 = { $call } не вдалося записати в тіло як calldata. Напишіть її в hex або дайте tracewing написати тіло.

# That is not a call written out.
tc159f8f5 = Це не виклик, записаний словами.

# No call named {name} is known here. Write its types with it, as transfer(address,uint256)(…), or add its ABI under Contract ABIs in Settings.
t61568d43 = Виклик з іменем { $name } тут невідомий. Напишіть разом із ним типи, як transfer(address,uint256)(…), або додайте його ABI у розділі «ABI контрактів» у Налаштуваннях.

# {name} is more than one call: {found}. Write the types with it, as {first}(…).
t48b75c8d = { $name } — це більше ніж один виклик: { $found }. Напишіть разом із ним типи, як { $first }(…).

# {types} are not types a call can be written with.
t544218b9 = { $types } — це не типи, якими можна записати виклик.

# The values do not fit {signature}: {why}
t14015b69 = Значення не підходять до { $signature }: { $why }

# Appearance: {now}. Click for {next}.
t332cfa6d = Вигляд: { $now }. Натисніть, щоб змінити на: { $next }.

# api.example.com, or *.example.com
tdc6588ac = api.example.com або *.example.com

# Choose
t73c66160 = Вибрати

# Choose the certificate's file.
t6c7ace6e = Виберіть файл сертифіката.

# Say which host it is for.
taf39b2e1 = Вкажіть, для якого хоста він.

# Not used: {why}
t3a0298f3 = Не використовується: { $why }

# Remove
t21a5901d = Видалити

# Client certificates
te3ce8812 = Клієнтські сертифікати

# For servers that ask the client for a certificate (mutual TLS): tracewing shows the one given for their host, in the app's place. A PEM certificate and its key without a passphrase; the files stay where they are. Each applies to the next connection.
tcd3f6864 = Для серверів, що вимагають сертифікат клієнта (взаємний TLS): tracewing пред'являє сертифікат, заданий для їхнього хоста, замість застосунку. Сертифікат PEM і його ключ без парольної фрази; файли лишаються на місці. Кожен діє з наступного з'єднання.

# None yet.
t786bc275 = Поки немає.

# Certificate…
t164f4d6c = Сертифікат…

# none chosen
t23b63bc7 = не вибрано

# Key…
t7aa250a8 = Ключ…

# in the certificate's file
t13bc7052 = у файлі сертифіката

# Add
t9dc3aa14 = Додати

# Method
t98a2c750 = Метод

# Status
t005ef20f = Статус

# App
t81f51acc = Застосунок

# Host
t302af18f = Хост

# Path
teb66e456 = Шлях

# Operation
t218e02a6 = Операція

# Size
ta6478e7c = Розмір

# Time
tdfe4e404 = Час

# Number
t61e9eac0 = Номер

# A body was not kept, so the two cannot be compared.
t9d6abb2a = Одне з тіл не збережено, тож порівняти їх не можна.

# {count} field differs | {count} fields differ
t6a8fff7f =
    { $n ->
        [one] Відрізняється { $count } поле
        [few] Відрізняються { $count } поля
        [many] Відрізняються { $count } полів
       *[other] Відрізняється { $count } поля
    }

# … {count} line the same | … {count} lines the same
tcc37e7ea =
    { $n ->
        [one] … { $count } однаковий рядок
        [few] … { $count } однакові рядки
        [many] … { $count } однакових рядків
       *[other] … { $count } однакового рядка
    }

# Nothing to compare.
t9f32d942 = Нічого порівнювати.

# URL
t7569633e = URL

# Protocol
t4a57ebd5 = Протокол

# Type
td155d06d = Тип

# Sizes
td1a4a19d = Розміри

# Body
t428a1095 = Тіло

# Headers
t334f5989 = Заголовки

# Query
t9e89e783 = Запит

# Request
t82454152 = Запит

# Response
t55a982fe = Відповідь

# Neither has headers kept.
tb43ef094 = Жоден не має збережених заголовків.

# Trailers
t7c44fa89 = Трейлери

# Neither URL has a query.
t4d8203e0 = Жодна URL не має query.

# Comparing…
ta8565d31 = Порівняння…

# No body
tfa4c2902 = Без тіла

# {kind}, its first {size} compared
t1d894c39 = { $kind }, порівняно перші { $size }

# Neither has a body.
tf2c47725 = Жоден не має тіла.

# The bodies are the same.
t85152e01 = Тіла однакові.

# {count} field differs. | {count} fields differ.
t0f651f77 =
    { $n ->
        [one] Відрізняється { $count } поле.
        [few] Відрізняються { $count } поля.
        [many] Відрізняються { $count } полів.
       *[other] Відрізняється { $count } поля.
    }

# The first {count} are listed.
taae62a3e = Показано лише перші { $count }.

# {count} line differs. | {count} lines differ.
tf57a3157 =
    { $n ->
        [one] Відрізняється { $count } рядок.
        [few] Відрізняються { $count } рядки.
        [many] Відрізняються { $count } рядків.
       *[other] Відрізняється { $count } рядка.
    }

# Fields
t3308bd9c = Поля

# Lines
t8a1a00dc = Рядки

# Compare
td57a0a54 = Порівняння

# #{a} beside #{b}: what only #{a} has is red, what only #{b} has is green.
t71ce2de9 = #{ $a } поруч із #{ $b }: те, що є лише в #{ $a }, — червоним, те, що є лише в #{ $b }, — зеленим.

# Swap
te519d36e = Поміняти місцями

# Back to Traffic
t644042d2 = Назад до Traffic

# "{method}" is not a method
tacdcf5cb = "{ $method }" не є методом

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = тіло запиту не збережено повністю, тому його не можна надіслати знову

# name: value, one a line
t4df37bea = назва: значення, по одному в рядку

# No flow is chosen
t550b0a98 = Потік не вибрано

# This flow: {e}
t296762f8 = Цей потік: { $e }

# The body is not a {message}: {e}
t3220a10a = Тіло не є { $message }: { $e }

# Protobuf {message}, written as JSON: it is sent as Protobuf.
t83b6cd6e = Protobuf { $message }, записаний як JSON: надсилається як Protobuf.

# Written out in the body and sent as calldata: {calls}
tcaecdf04 = Записано в тілі й надіслано як calldata: { $calls }

# {names} is the client's to work out, so what was written for it did not go. | {names} are the client's to work out, so what was written for them did not go.
te85cbcea =
    { $n ->
        [one] { $names } клієнт задає сам, тому написане для нього не пішло.
        [few] { $names } клієнт задає сам, тому написане для них не пішло.
        [many] { $names } клієнт задає сам, тому написане для них не пішло.
       *[other] { $names } клієнт задає сам, тому написане для них не пішло.
    }

# Stopped: no answer was read. What reached the server shows in Traffic.
t5a78825c = Зупинено: відповідь не прочитано. Те, що дійшло до сервера, видно в Traffic.

# Sending…
te946e7bf = Надсилання…

# Send
tf28e14cf = Надіслати

# Stop
t4b7f7705 = Зупинити

# New
t8b242db1 = Новий

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … ще байтів: { $count }; усі вони є в потоці на вкладці «Трафік».

# Body · {message}
taa3cc3ff = Тіло · { $message }

# Compose
t18049f93 = Створити запит

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Ваш власний запит або надісланий повторно. Він проходить через tracewing, тож правила застосовуються, і він також з’являється у «Трафіку».

# {version} both sides
t961a5ea0 = { $version } з обох боків

# {client} to the proxy, {server} to the server
t3d237646 = { $client } до проксі, { $server } до сервера

# Queued
t9e437c74 = У черзі

# Connect
tdb89a899 = Підключення

# Waiting
tc9450e48 = Очікування

# Download
t6964bb19 = Завантаження

# No file picker: {e}
t6935fbd3 = Немає вікна вибору файлів: { $e }

# Writing the HAR…
t725a0e89 = Запис HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
        [one] Записано { $n } потік у { $path }
        [few] Записано { $n } потоки у { $path }
        [many] Записано { $n } потоків у { $path }
       *[other] Записано { $n } потоку у { $path }
    }

# The HAR was not written: {e}
tef8ec569 = HAR не записано: { $e }

# Open
t538b10e9 = Відкрити

# Reading {path}…
tc0e7c8f4 = Читання { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
        [one] , { $n } запис пропущено
        [few] , { $n } записи пропущено
        [many] , { $n } записів пропущено
       *[other] , { $n } запису пропущено
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
        [one] Прочитано { $n } потік ({ $format })
        [few] Прочитано { $n } потоки ({ $format })
        [many] Прочитано { $n } потоків ({ $format })
       *[other] Прочитано { $n } потоку ({ $format })
    }

# The file was not read: {e}
t7f49510f = Файл не прочитано: { $e }

# it is no longer kept
ta7b4c380 = він більше не зберігається

# {size} saved to {path}
tc6932467 = { $size } збережено у { $path }

# The body was not saved: {e}
t189900b4 = Тіло не збережено: { $e }

# Fiddler session
t9f5c89a3 = Сеанс Fiddler

# Charles session
tb4f56f2b = Сеанс Charles

# All
t75afe364 = Усі

# Errors
te9ed60a6 = Помилки

# Media
td10fcaa3 = Медіа

# Rule: {rule}
teca2074f = Правило: { $rule }

# Events
t003420a4 = Події

# JSON lines
te8ce2db0 = Рядки JSON

# Not decrypted
t8090a1ba = Не розшифровано

# open
td35ec4c9 = відкрито

# Reading the request…
t51c96267 = Читання запиту…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = Тіло запиту не збережено, тому його запит не можна показати.

# An operation without a name
t38c4d30b = Операція без назви

# {n} of {count}: {name}
t53045460 = { $n } з { $count }: { $name }

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Persisted query: текст зберігає сервер, а запит надсилає лише його хеш.

# Variables
t7996f062 = Змінні

# Cut
t58e25863 = Вирізати

# Copy
t658f3664 = Копіювати

# Paste
tb34a0288 = Вставити

# Select all
t6af21072 = Виділити все

# This flow is gone.
t3343999e = Цього потоку більше немає.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Цей хост не розшифровується, тому не видно, що він надсилає.

# Still coming: it shows once it ends.
t42be95fb = Ще надходить: з’явиться, коли закінчиться.

# No body.
t31e4d044 = Тіла немає.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Це тіло не збережено: диск майже заповнений або забагато даних чекало на запис.

# This body was not kept.
te46616b5 = Це тіло не збережено.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Цього тіла більше немає: його видалено разом із найстарішими потоками, або його не вдається прочитати.

# Cannot open this body: {e}
t620a039b = Не вдається відкрити це тіло: { $e }

# Inspector
t4def410e = Інспектор

# Choose a flow to see it here
tc7be91e8 = Виберіть потік, щоб переглянути його тут

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Не розшифровано. Нові з’єднання з цим хостом проходять непрочитаними.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Якщо застосунок не працює з цим хостом, можливо, він закріплює свої сертифікати (pinning).

# Make a rule
t0ac0e062 = Створити правило

# Decrypt this host
tb7367e2c = Розшифровувати цей хост

# Do not decrypt this host
t5fa113f6 = Не розшифровувати цей хост

# Request and response
t04e060e6 = Запит і відповідь

# Request and response one at a time
t70397119 = Запит і відповідь по черзі

# Request and response side by side
t3d35ce69 = Запит і відповідь поруч

# Request above the response
t83c63123 = Запит над відповіддю

# Table
td70ceb7f = Таблиця

# Pretty
t7e09470d = Формат

# Raw
ta689aa91 = Вихідний

# Picture · {size}
te97ed633 = Зображення · { $size }

# Messages {count}
te61b841a = Повідомлення { $count }

# Headers {headers}
t04b0146d = Заголовки { $headers }

# Cookies {cookies}
td58eb84b = Cookie { $cookies }

# Query {query}
t4756f9f5 = Параметри { $query }

# Form {form}
tfa6e7203 = Форма { $form }

# Connection
t09ea4ff9 = З’єднання

# Save body
te2c44424 = Зберегти тіло

# Send again
tf5910557 = Надіслати знову

# Edit and send
tf837e542 = Змінити й надіслати

# Copied
t8dc21305 = Скопійовано

# Copy as cURL
t866d1e3e = Копіювати як cURL

# Click another flow…
t59253578 = Натисніть на інший потік…

# Compare with…
t58dfe040 = Порівняти з…

# Compare with the call before
tef15286c = Порівняти з попереднім викликом

# Copy as…
t9c1ad448 = Копіювати як…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Це зображення не вдається показати. Збережіть тіло, щоб переглянути його.

# Opening…
tc40f2321 = Відкриття…

# No response
t8c2c37b1 = Немає відповіді

# Tunnel, not decrypted
tce693254 = Тунель, не розшифровано

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# RPC: {method}
tf8162891 = RPC: { $method }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Через з’єднання { $id }, створене раніше: на підключення час не витрачено.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Не знайдено: можливо, процес завершився або належить іншому користувачеві

# On another device
t2b964a37 = На іншому пристрої

# CLIENT
t6d3cd99e = КЛІЄНТ

# Device
tbdcbb413 = Пристрій

# Program
tc4253b6b = Програма

# Address
t700560f3 = Адреса

# SERVER
te3b27612 = СЕРВЕР

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = Відомості про це з’єднання більше не зберігаються: їх видалено разом зі старішими потоками.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Не розшифровано: tracewing передав його далі, не переглядаючи.

# No connection was made.
tf348eb92 = З’єднання не встановлено.

# None: it was answered without one.
tac71808e = Немає: відповідь надано без з’єднання.

# {id}, given up during TLS
t0ec66a8c = { $id }, перервано під час TLS

# {id}, made for this flow
t7fe32cfc = { $id }, створено для цього потоку

# {id}, made before and used again
ta904c078 = { $id }, створено раніше й використано повторно

# Server
t6d7302f2 = Сервер

# None: plain HTTP
t05606349 = Немає: звичайний HTTP

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = СЕРВЕР НЕ НАДАВ СЕРТИФІКАТІВ

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = ЩО НАДАВ СЕРВЕР, ПЕРШИМ — ЙОГО ВЛАСНИЙ

# NO CERTIFICATES
te6f31dce = НЕМАЄ СЕРТИФІКАТІВ

# CERTIFICATE
t1c902790 = СЕРТИФІКАТ

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
        [one] { $n } СЕРТИФІКАТ, ПЕРШИМ — СЕРВЕРА
        [few] { $n } СЕРТИФІКАТИ, ПЕРШИМ — СЕРВЕРА
        [many] { $n } СЕРТИФІКАТІВ, ПЕРШИМ — СЕРВЕРА
       *[other] { $n } СЕРТИФІКАТА, ПЕРШИМ — СЕРВЕРА
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing не став продовжувати роботу з цим сервером: { $error }. Якщо застосунок довіряє йому іншим способом, наприклад через корпоративний CA, виберіть «Не розшифровувати цей хост», і застосунок сам перевірить сервер.

# Cipher
t5b3bed6a = Шифр

# none
tada7afdb = немає

# not valid until {when}
t0a14b2d8 = недійсний до { $when }

# expired {when}
t932538c2 = термін дії сплив { $when }

# expires {when}
t679bcb51 = термін дії спливає { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
        [one] { $names } і ще { $n }
        [few] { $names } і ще { $n }
        [many] { $names } і ще { $n }
       *[other] { $names } і ще { $n }
    }

# Issued by {issuer}
t15a1ce5a = Видавець { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Дійсний з { $from } до { $to } · { $when }

# Names {names}
tda75e459 = Імена { $names }

# Serial {serial}
td1f0d98b = Серійний номер { $serial }

# These headers were not kept.
tb8739fec = Ці заголовки не збережено.

# No headers.
tf3dacb5e = Заголовків немає.

# Trailers, sent after the body
t3447e9de = Трейлери, надіслані після тіла

# File {filename} · {kind} · {size}
t6dbd239f = Файл { $filename } · { $kind } · { $size }

# no type
t902a6ece = без типу

# No cookies were sent.
t58e85ab2 = Cookie не надсилалися.

# No cookies were set.
tf57dbabc = Cookie не встановлювалися.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · підпис не перевірено

# JWT · signature not checked
t05f8b533 = JWT · підпис не перевірено

# Header
t114f9980 = Заголовок

# Payload
ta0639745 = Payload

# Light
t8ad2f8cf = Світла

# Dark
t45570065 = Темна

# System
t8f3184bc = Системна

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
        [one] потокова передача, { $count } фрагмент
        [few] потокова передача, { $count } фрагменти
        [many] потокова передача, { $count } фрагментів
       *[other] потокова передача, { $count } фрагмента
    }

# stopped: {stop}
tfa48fb30 = зупинка: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = токени: { $input } на вході · { $output } на виході

# Reading the call…
t33b52a3c = Читання виклику…

# Answer
tf94ea0ff = Відповідь

# Thinking: {thinking}
t737ece33 = Міркування: { $thinking }

# Calls {name}{id}
t017882e4 = Викликає { $name }{ $id }

# Nothing yet.
td9b71d16 = Поки нічого.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
        [one] { $text }… і ще { $count } символ
        [few] { $text }… і ще { $count } символи
        [many] { $text }… і ще { $count } символів
       *[other] { $text }… і ще { $count } символу
    }

# Copy the answer
t69975ed7 = Копіювати відповідь

# Asked
t4e3af9c3 = Запитано

# Tools offered: {tools}
tb871bcf4 = Запропоновані інструменти: { $tools }

# Not kept
tba30b5fa = Не збережено

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Це повідомлення не збережено: забагато даних чекало на запис або диск майже заповнений.

# This message is empty.
tdb0ec01f = Це повідомлення порожнє.

# No messages yet
t51e7624c = Поки немає повідомлень

# {said} kept, of {size}
t3558745a = Збережено { $said } з { $size }

# The socket has closed.
t8d883a0f = Сокет закрито.

# Too many are still waiting to go. Try again.
tbea55dfd = Забагато повідомлень ще чекають на відправлення. Спробуйте ще раз.

# To the server
te1ef193a = Серверу

# To the app
t1bcabce4 = Застосунку

# Layouts
t17809004 = Розкладки

# Keep the panels as they are now under a name, to come back to them.
t2b6aeb69 = Збережіть поточне розташування панелей під назвою, щоб повернутися до нього.

# Keep
t555cbe68 = Зберегти

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } не є цілим числом

# Latency
t23a45ef9 = Затримка

# Down
ta4995035 = Завантаження

# Up
t42f35940 = Вивантаження

# Loss
t69db303a = Втрати

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Укажіть затримку, швидкість або втрати, щоб сповільнити мережу

# Latency, ms
tefe18ab9 = Затримка, ms

# Down, kbit/s
t08c4ae33 = Завантаження, kbit/s

# Up, kbit/s
td55cfe9a = Вивантаження, kbit/s

# Loss, %
tea4af439 = Втрати, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Лише ці хости, наприклад *.example.com; якщо порожньо — усі

# Not kept for next time: {e}
tdce4d67c = Не збережено на наступний раз: { $e }

# Slow network
td51ddfce = Повільна мережа

# Off: traffic goes as fast as it can
tf0f34db0 = Вимкнено: трафік іде з максимальною швидкістю

# Custom
t9525b78e = Власний

# Only these hosts
tfe16992b = Лише ці хости

# Apply
tca77bbdb = Застосувати

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Діє з наступного запиту. Затримка додається до кожного обміну туди й назад; швидкість — це один канал, спільний для всіх сповільнених потоків; втрачений фрагмент чекає на повторне надсилання, як у TCP. Хости поза списком не сповільнюються.

# Header line {line} has no ":"
t891b076d = У рядку заголовка { $line } немає ":"

# Header line {line} has no name
te3247b96 = У рядку заголовка { $line } немає назви

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
        [one] { $n } байт, що не є текстом: передається без змін.
        [few] { $n } байти, що не є текстом: передаються без змін.
        [many] { $n } байтів, що не є текстом: передаються без змін.
       *[other] { $n } байта, що не є текстом: передаються без змін.
    }

# Nothing is paused
t8cf68b4c = Нічого не призупинено

# "{status}" is not a status
t40a529af = "{ $status }" не є статусом

# REQUEST
t1f3f6dd2 = ЗАПИТ

# RESPONSE
tc85f77de = ВІДПОВІДЬ

# MESSAGE TO THE SERVER
t270a6209 = ПОВІДОМЛЕННЯ СЕРВЕРУ

# MESSAGE TO THE APP
t7d928b05 = ПОВІДОМЛЕННЯ ЗАСТОСУНКУ

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Нічого не призупинено. Додайте правило «Призупинити» на екрані «Правила», щоб зупиняти трафік тут.

# {kind} on {url}
t1204e859 = { $kind } у { $url }

# for {method} {url}
t093a86f6 = для { $method } { $url }

# The request, before the server gets it
tc840fed6 = Запит до того, як його отримає сервер

# The response, before the app gets it
t6683d72a = Відповідь до того, як її отримає застосунок

# A socket's message, before the server gets it
tda2ebc5d = Повідомлення сокета до того, як його отримає сервер

# A socket's message, before the app gets it
tbce07e9d = Повідомлення сокета до того, як його отримає застосунок

# Message
tae0ed984 = Повідомлення

# Send as it came
tb593e730 = Надіслати без змін

# Drop
t108524ac = Відкинути

# Paused
tde281feb = Призупинені

# Nothing is waiting.
tea613769 = Нічого не очікує.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
        [one] { $n } чекає на надсилання.
        [few] { $n } чекають на надсилання.
        [many] { $n } чекають на надсилання.
       *[other] { $n } чекає на надсилання.
    }

# Send all as they came
tdce25640 = Надіслати всі без змін

# The list was not saved: {e}
tfceba380 = Список не збережено: { $e }

# {count} service: {names} | {count} services: {names}
tc1b8764c =
    { $n ->
        [one] { $count } сервіс: { $names }
        [few] { $count } сервіси: { $names }
        [many] { $count } сервісів: { $names }
       *[other] { $count } сервісу: { $names }
    }

# https://api.example.com or http://localhost:50051
t14f9744c = https://api.example.com або http://localhost:50051

# Reading…
t6aad8485 = Читання…

# {count} file | {count} files
tde674bd8 =
    { $n ->
        [one] { $count } файл
        [few] { $count } файли
        [many] { $count } файлів
       *[other] { $count } файлу
    }

# {count} message | {count} messages
t8a1cd2f6 =
    { $n ->
        [one] { $count } повідомлення
        [few] { $count } повідомлення
        [many] { $count } повідомлень
       *[other] { $count } повідомлення
    }

# {count} service | {count} services
t51f71902 =
    { $n ->
        [one] { $count } сервіс
        [few] { $count } сервіси
        [many] { $count } сервісів
       *[other] { $count } сервісу
    }

# and {count} more problem | and {count} more problems
t591ce5ea =
    { $n ->
        [one] і ще { $count } проблема
        [few] і ще { $count } проблеми
        [many] і ще { $count } проблем
       *[other] і ще { $count } проблеми
    }

# Protobuf schemas
ta4401d1a = Схеми Protobuf

# Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
te5f4f07c = Теки з файлами .proto або descriptor set, створені через protoc -o, щоб показувати тіла Protobuf і gRPC з назвами полів. Виклик gRPC знаходить свої повідомлення за шляхом, інший Protobuf — за messageType у його типі. Їх перечитують, коли вони змінюються.

# Add a folder…
tb5930e43 = Додати теку…

# Add files…
ted08dbe5 = Додати файли…

# Read again
t2f0dcc5d = Перечитати

# Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.
tf6948a5a = Або попросіть файли в сервера з gRPC reflection. https:// іде через tracewing; http:// — gRPC без TLS, напряму до сервера.

# Asking…
tf9cf22fe = Запит…

# Fetch
t533a3183 = Отримати

# Find a message
te6f78626 = Знайти повідомлення

# Read as {message}
t87b6a380 = Читати як { $message }

# Read as…
t1b90af95 = Читати як…

# Requests to {path} on {host} are read as the message chosen.
t8826d766 = Запити до { $path } на { $host } читаються як вибране повідомлення.

# Answers from {path} on {host} are read as the message chosen.
tbbe012da = Відповіді від { $path } на { $host } читаються як вибране повідомлення.

# Automatic (gRPC path or messageType)
t134c9a06 = Автоматично (шлях gRPC або messageType)

# and {count} more: type to narrow them | and {count} more: type to narrow them
t70ff68f9 =
    { $n ->
        [one] і ще { $count }: введіть, щоб звузити
        [few] і ще { $count }: введіть, щоб звузити
        [many] і ще { $count }: введіть, щоб звузити
       *[other] і ще { $count }: введіть, щоб звузити
    }

# Nothing to read in this
t92a8694c = Тут нічого прочитати

# Write the server's address first
t4f68f91a = Спершу введіть адресу сервера

# "{address}" is not a server's address
t8c4d03a6 = «{ $address }» — не адреса сервера

# The server has no services to describe
tfa5b7ad8 = Сервер не має сервісів для опису

# The server sent no files for its services
tdca19bdc = Сервер не надіслав файлів своїх сервісів

# The server answered {status}: it may not offer gRPC reflection
t345fdb40 = Сервер відповів { $status }: можливо, він не має gRPC reflection

# The server answered {code}: it may not offer gRPC reflection. {message}
tb629329e = Сервер відповів { $code }: можливо, він не має gRPC reflection. { $message }

# The server answered nothing: it may not offer gRPC reflection
t6bbb6207 = Сервер нічого не відповів: можливо, він не має gRPC reflection

# The password is kept in the system's keychain.
t3a48373c = Пароль зберігається в системній в'язці ключів.

# The password is kept in relay.toml, which only you can read.
tf84cb18b = Пароль зберігається в relay.toml, який можете читати лише ви.

# The password is kept in relay.toml, which only you can read: the keychain cannot be used ({why}).
t5a08b6d0 = Пароль зберігається в relay.toml, який можете читати лише ви: в'язку ключів використати не можна ({ $why }).

# New connections go through the {kind} proxy {address}.
tb5f1ef96 = Нові з'єднання йдуть через { $kind }-проксі { $address }.

# Connections go straight to servers.
t4a64a08f = З'єднання йдуть до серверів напряму.

# relay.toml cannot be used, so connections go straight: {problem}
tbc6a2da3 = relay.toml не можна використати, тож з'єднання йдуть напряму: { $problem }

# http://proxy.example.com:8080 or socks5://proxy.example.com:1080
tb945a053 = http://proxy.example.com:8080 або socks5://proxy.example.com:1080

# Go through another proxy
tc67dfdb1 = Іти через інший проксі

# Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080; the password is then kept in the system's keychain, not shown again. Empty, tracewing goes straight to servers.
t5c30a22f = Якщо мережа компанії випускає назовні лише через свій проксі, tracewing іде до серверів через нього. Застосунки й далі надсилають трафік до tracewing. Ім'я користувача й пароль пишуться в адресі, як http://ім'я:пароль@proxy:8080; потім пароль зберігається в системній в'язці ключів і більше не показується. Порожньо — tracewing іде до серверів напряму.

# Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host.
tcbc7d298 = Хости, до яких іти напряму, як у NO_PROXY: example.com — він сам і все під ним, * — будь-який хост.

# Sent by the server, not asked for
td717c770 = Надіслано сервером самостійно, не у відповідь на запит

# any
t2c29f04d = будь-який

# Failed
t5579c3d4 = Не вдалося

# The server answered 200: a call that fails says so in the body.
ta74e5f73 = Сервер відповів 200: невдалий виклик повідомляє про це в тілі.

# {count} item | {count} items
t534e422e =
    { $n ->
        [one] { $count } елемент
        [few] { $count } елементи
        [many] { $count } елементів
       *[other] { $count } елементів
    }

# {count} field | {count} fields
t0039810a =
    { $n ->
        [one] { $count } поле
        [few] { $count } поля
        [many] { $count } полів
       *[other] { $count } полів
    }

# and {count} more
tbdb1ff6a = і ще { $count }

# {count} byte that does not read as one | {count} bytes that do not read as one
ta7ee514c =
    { $n ->
        [one] { $count } байт, який не читається як транзакція
        [few] { $count } байти, які не читаються як транзакція
        [many] { $count } байтів, які не читаються як транзакція
       *[other] { $count } байтів, які не читаються як транзакція
    }

# {program}, {count} bytes
t1ce6ad3b = { $program }, { $count } байтів

# succeeded
t6b166ff2 = успішно

# failed: it was mined, and what it asked for did not happen
t46ea918c = не вдалося: вона потрапила в блок, а те, про що просила, не сталося

# none: it makes a contract
t188b58f6 = немає: створює контракт

# {count} byte, which the signature does not say how to read | {count} bytes, which the signature does not say how to read
t36ffce30 =
    { $n ->
        [one] { $count } байт, який підпис не пояснює, як читати
        [few] { $count } байти, які підпис не пояснює, як читати
        [many] { $count } байтів, які підпис не пояснює, як читати
       *[other] { $count } байтів, які підпис не пояснює, як читати
    }

# {count} place | {count} places
t0965779e =
    { $n ->
        [one] { $count } місце
        [few] { $count } місця
        [many] { $count } місць
       *[other] { $count } місць
    }

# {count} blob | {count} blobs
t95369f92 =
    { $n ->
        [one] { $count } блоб
        [few] { $count } блоби
        [many] { $count } блобів
       *[other] { $count } блобів
    }

# 0, the kind from before EIP-2718
tc2fde48e = 0, вид до EIP-2718

# The bodies were not kept, so what the call sent cannot be shown.
teaf455ca = Тіла не збережено, тому не можна показати, що надіслав виклик.

# and {count} more calls
tf0ac7103 = і ще { $count } викликів

# {count} function | {count} functions
t9ffc3c7c =
    { $n ->
        [one] { $count } функція
        [few] { $count } функції
        [many] { $count } функцій
       *[other] { $count } функцій
    }

# {count} event | {count} events
t4dd03992 =
    { $n ->
        [one] { $count } подія
        [few] { $count } події
        [many] { $count } подій
       *[other] { $count } подій
    }

# {count} error | {count} errors
t4fba4c4e =
    { $n ->
        [one] { $count } помилка
        [few] { $count } помилки
        [many] { $count } помилок
       *[other] { $count } помилок
    }

# Contract ABIs
t08fb00f2 = ABI контрактів

# Folders or files of ABI JSON, as solc, Hardhat and Foundry write it, to name the functions, events and errors in a JSON-RPC call instead of showing four bytes. They are read again when they change. Nothing is sent anywhere: a signature is worked out from the ABI on this machine.
tfadffbd2 = Теки або файли ABI JSON, як їх пишуть solc, Hardhat і Foundry, щоб називати функції, події та помилки у виклику JSON-RPC замість чотирьох байтів. Їх читають знову, коли вони змінюються. Нікуди нічого не надсилається: підпис обчислюється з ABI на цій машині.

# Block
tbd074002 = Заблокувати

# Answer from a file
td8edab50 = Відповісти з файлу

# Send to another host
te3cd85d7 = Надіслати на інший хост

# Set a request header
tfad4aaaa = Задати заголовок запиту

# Set a response header
tcbbc20d0 = Задати заголовок відповіді

# Remove a request header
t5228dd84 = Видалити заголовок запиту

# Remove a response header
teeb5735a = Видалити заголовок відповіді

# Set a query parameter
t73d9419b = Задати параметр запиту

# Remove a query parameter
t12633da9 = Видалити параметр запиту

# Change the request body
t41101201 = Змінити тіло запиту

# Change the response body
tdd259b73 = Змінити тіло відповіді

# Change the status
t4bbdfba8 = Змінити статус

# Pause the request
t40ecc39b = Призупинити запит

# Pause the response
t18a34799 = Призупинити відповідь

# Pause a socket's messages to the server
te5d065fe = Призупиняти повідомлення сокета серверу

# Pause a socket's messages to the app
t482173e8 = Призупиняти повідомлення сокета застосунку

# Run a script
t65a52970 = Запустити скрипт

# Status to answer with, like 403
t8f3ebb6c = Статус для відповіді, наприклад 403

# Full path of a file or folder
t9fb9ae4c = Повний шлях до файлу або папки

# Full path of a .js file with onRequest or onResponse
t01bacb53 = Повний шлях до файлу .js з onRequest або onResponse

# Where to, like http://localhost:3000
tdfedd004 = Куди, наприклад http://localhost:3000

# Header's name
t80793085 = Назва заголовка

# Its value
t43968778 = Його значення

# Parameter's name
t313c8e35 = Назва параметра

# Text to find, or empty for the whole body
t8255fc87 = Текст для пошуку, або порожньо для всього тіла

# What to put in its place
t03671fd5 = Що поставити на його місце

# Status the client gets, like 500
t5e4d3088 = Статус, який отримає клієнт, наприклад 500

# "{first}" is not a status
td43128d3 = "{ $first }" не є статусом

# Block with {status}
t1727e984 = Заблокувати зі статусом { $status }

# Answer from {path}
taefc08fa = Відповісти з { $path }

# Send to {to}
t5e256873 = Надіслати на { $to }

# Request: {header}: {value}
taa230766 = Запит: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Відповідь: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Запит: без { $header }

# Response: no {header}
t23960a2e = Відповідь: без { $header }

# Query: {name}={value}
t0ee6c1c0 = Query: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Query: без { $name }

# Status {status}
tad048e53 = Статус { $status }

# Run {path}
t2f9fde3f = Запустити { $path }

# Request body: {replace}
te4856c36 = Тіло запиту: { $replace }

# Response body: {replace}
t8600dc42 = Тіло відповіді: { $replace }

# Request body: {find} → {replace}
taec471a9 = Тіло запиту: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Тіло відповіді: { $find } → { $replace }

# Name
t0fe07306 = Назва

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, наприклад api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Метод, або порожньо для всіх

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
        [one] Записано { $n } правило у { $path }
        [few] Записано { $n } правила у { $path }
        [many] Записано { $n } правил у { $path }
       *[other] Записано { $n } правила у { $path }
    }

# The rules were not written: {e}
te7ac257f = Правила не записано: { $e }

# Import
t573e0af4 = Імпорт

# {path} could not be read: {e}
t32a3ce94 = Не вдалося прочитати { $path }: { $e }

# Nothing was imported: {e}
tb9f7257a = Нічого не імпортовано: { $e }

# Failed: {why}
tfce667e2 = Помилка: { $why }

# Ran once. Last said: {last} | Ran {n} times. Last said: {last}
t3e82a85a =
    { $n ->
        [one] Запущено { $n } раз. Останній вивід: { $last }
        [few] Запущено { $n } рази. Останній вивід: { $last }
        [many] Запущено { $n } разів. Останній вивід: { $last }
       *[other] Запущено { $n } рази. Останній вивід: { $last }
    }

# Ran once. | Ran {n} times.
t47b9c718 =
    { $n ->
        [one] Запущено { $n } раз.
        [few] Запущено { $n } рази.
        [many] Запущено { $n } разів.
       *[other] Запущено { $n } рази.
    }

# Edit
tc2c76cb1 = Змінити

# Delete
t5797ea6a = Видалити

# Change this rule
t9160f393 = Змінити це правило

# A new rule
t55a722a0 = Нове правило

# It waits under Paused until you send it on, changed or not.
tca566a57 = Він чекає в «Призупинених», доки ви не надішлете його далі, зі змінами чи без.

# Save
t4d2d5d68 = Зберегти

# Cancel
t35afca3b = Скасувати

# Rules
t428b2d80 = Правила

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Правила діють з наступного запиту, і поки запис на паузі. Згори донизу: перемагає перше блокування, файл чи хост, а всі інші відповідні правила застосовуються по черзі. Правило, що змінює тіло чи статус, змінює те, що надіслав сервер.

# Import rules…
t5edc205d = Імпортувати правила…

# Export rules…
t6e09ca20 = Експортувати правила…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
        [one] Додано { $n } правило
        [few] Додано { $n } правила
        [many] Додано { $n } правил
       *[other] Додано { $n } правила
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
        [one] { $n } уже є
        [few] { $n } уже є
        [many] { $n } уже є
       *[other] { $n } уже є
    }

# left out: {why}
ta81e95e2 = пропущено: { $why }

# Copy all
t6aef5c37 = Копіювати все

# Too large to copy all
t2fe8ff9f = Завелике, щоб копіювати все

# No flows
t6dc463eb = Немає потоків

# Name it to keep it
t52c1a402 = Дайте назву, щоб зберегти

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · записано іншою версією, тому не відкривається

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
        [one] { $count } потік · { $size }
        [few] { $count } потоки · { $size }
        [many] { $count } потоків · { $size }
       *[other] { $count } потоку · { $size }
    }

# Recording now
t3cb3e6fc = Іде запис

# Export HAR
t4d6458cc = Експортувати HAR

# Delete for good?
ta7737f13 = Видалити назавжди?

# Sessions
tb69a8aec = Сеанси

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Сеанс із назвою зберігається, доки ви його не видалите. З інших залишаються { $kept } найновіших.

# Working…
td5507854 = Виконується…

# Open a session file
tae8622fc = Відкрити файл сеансу

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, Fiddler .saz або Charles .chlsj

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = В експортованих HAR немає cookie, токенів, ключів і паролів — ні в заголовках, ні в URL, ні в тілах

# Other devices
t9e7d8362 = Інші пристрої

# Let other devices use the proxy
t13e626b3 = Дозволити іншим пристроям використовувати проксі

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = Тоді телефони й інші комп’ютери у вашій мережі зможуть надсилати сюди свій трафік. Як і будь-хто інший у цій мережі, тому не вмикайте це в мережі, якій ви не довіряєте.

# Servers
tcd0cf113 = Сервери

# Trust what this computer trusts
tfea2d91b = Довіряти тому, чому довіряє цей комп’ютер

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = До серверів із сертифікатом від центру сертифікації, установленого на цьому комп’ютері (наприклад, корпоративного), підключення відбувається так само, як у браузерах цього комп’ютера. Якщо вимкнено, довіряються лише публічні центри сертифікації. Набуває чинності після перезапуску tracewing.

# AI tools
t294d7ea4 = Інструменти ШІ

# Let AI tools see tokens and cookies
te1d26d3c = Дозволити інструментам ШІ бачити токени й cookie

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = Інструменти ШІ читають записаний трафік через MCP-сервер tracewing. Якщо вимкнено, значення Authorization, Cookie і Set-Cookie їм не передаються. Діє одразу.

# Let AI tools add rules and send requests
t663de1ba = Дозволити ШІ-інструментам додавати правила й надсилати запити

# Through the MCP server, an AI tool may then add and remove rules, which change traffic as yours do, and send requests through tracewing, again or changed, which reach their servers. Off, it can only read. Applies at once.
t46d53486 = Через MCP-сервер ШІ-інструмент зможе додавати й вилучати правила, які змінюють трафік, як ваші, і надсилати запити через tracewing, повторно або змінені, — вони доходять до своїх серверів. Якщо вимкнено, він може лише читати. Діє одразу.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Не пускати застосунки на HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = HTTP/3 працює поверх UDP, який жоден проксі не передає, тож застосунок, що використовує його, обходить tracewing. Це налаштування прибирає з відповідей серверів пропозиції HTTP/3, і застосунки залишаються на HTTP/2. Потоки все одно показують пропозиції так, як їх надіслали сервери.

# Size of a session
ta20fa102 = Розмір сеансу

# Past this, the oldest flows are dropped.
td2914f46 = Понад цей розмір найстаріші потоки видаляються.

# Room to leave on the disk
t8674a3b3 = Скільки місця залишати на диску

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Коли вільного місця менше, потоки й далі з’являються у списку, але тіла не зберігаються.

# Earlier sessions to keep
td356194a = Скільки попередніх сеансів зберігати

# Older ones are deleted when a new session starts.
t66556513 = Старіші видаляються, коли починається новий сеанс.

# Bodies
tdb4d0409 = Тіла

# Most kept of one body
t777508dd = Найбільше, що зберігається з одного тіла

# It still reaches the client whole.
t386fee28 = Клієнт однаково отримує його повністю.

# Most unpacked for viewing
tc3fe2efd = Найбільше, що розпаковується для перегляду

# Stops a small download that unpacks to many GB.
tf9c49515 = Захищає від невеликого завантаження, що розпаковується в багато GB.

# Small body size
ta8996af1 = Розмір малого тіла

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Менші тіла чекають у пам’яті й спільно використовують один файл. Більші отримують власний файл під час надходження.

# Compress plain text on disk
tee503c2e = Стискати звичайний текст на диску

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Займає приблизно вдвічі менше місця; пропускається, коли трафік надто швидкий для цього.

# Compression level
td386c9ef = Рівень стиснення

# 1 to 19. Higher is smaller and slower.
td42a987d = Від 1 до 19. Вищий — менший розмір і повільніше.

# Search
tc646a2c9 = Пошук

# Search URLs
t0c3e0b3d = Пошук в URL

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Знаходить будь-яку частину URL під час введення. Майже нічого не коштує.

# Search bodies
t8eec0293 = Пошук у тілах

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Знаходить слова в тілах. Може зайняти стільки ж місця на диску, скільки самі тіла.

# Searched part of each body
tab293b21 = Частина кожного тіла для пошуку

# Words past this are not found. More takes more disk.
t0bf65f4a = Слова за цією межею не знаходяться. Більше значення займає більше місця на диску.

# Under heavy traffic
ta99888cb = За великого трафіку

# Memory for bodies waiting to be written
t7d6ff799 = Пам’ять для тіл, що чекають на запис

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Понад цей обсяг потоки з’являються у списку без тіл. Трафік ніколи не затримується.

# Memory for large bodies waiting to be written
t07b9de08 = Пам’ять для великих тіл, що чекають на запис

# Past this, large bodies stop being kept.
tfe61a4f3 = Понад цей обсяг великі тіла більше не зберігаються.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }" не є цілим числом

# {key}: must not be negative
t9c4debaa = { $key }: не може бути від’ємним

# After a restart
t58dcea2d = Після перезапуску

# Every setting is back to its default.
t763785e7 = Усі налаштування повернуто до типових значень.

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d =
    { $n ->
        [one] { $n } зміна набуде чинності після перезапуску tracewing.
        [few] { $n } зміни наберуть чинності після перезапуску tracewing.
        [many] { $n } змін наберуть чинності після перезапуску tracewing.
       *[other] { $n } зміни наберуть чинності після перезапуску tracewing.
    }

# Settings
t4b058728 = Налаштування

# Undo
t71fd4acf = Відмінити

# Back to defaults
ta3162a13 = Відновити типові

# Hosts not to decrypt
tb457f0b3 = Хости, які не розшифровувати

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = По одному в рядку: хост або *.example.com для нього й усього під ним. Їхній трафік проходить непрочитаним. Список застосовується одразу.

# Authorities trusted for servers
t0a308a8a = Довірені центри сертифікації для серверів

# tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in {folder}. They are read when tracewing starts.
t571051f9 = tracewing довіряє серверам, за які ручаються публічні центри сертифікації та центри, яким довіряє цей комп'ютер. Також він довіряє цим: власному центру компанії або вашому власному, покладеному як файл .pem у { $folder }. Вони читаються під час запуску tracewing.

# Left out: {why}
t4a567d82 = Пропущено: { $why }

# The folder has changed since tracewing started: restart it to use what it holds now.
t65f2d800 = Папка змінилася після запуску tracewing: перезапустіть його, щоб використати її поточний вміст.

# Open the folder
tc7d7e212 = Відкрити папку

# Read it again
td72efd5e = Прочитати знову

# TLS keys for Wireshark
tcb4394af = TLS-ключі для Wireshark

# Writes the secrets of each HTTPS connection, from apps to tracewing and from it to servers, to a file Wireshark reads: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Anyone with the file can read the traffic it covers. Off again when tracewing starts, unless SSLKEYLOGFILE names a file.
t2f92ac10 = Записує секрети кожного HTTPS-з'єднання — від застосунків до tracewing і від нього до серверів — у файл, який читає Wireshark: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Будь-хто з цим файлом може прочитати охоплений ним трафік. Знову вимикається під час запуску tracewing, якщо SSLKEYLOGFILE не вказує файл.

# Cannot write to it: {why}
t956ecc4b = Не вдається записати в нього: { $why }

# Copy the path
t095d064e = Скопіювати шлях

# Language
t9a73db9b = Мова

# System ({system_name})
tb5c7a1d4 = Системна ({ $system_name })

# Applies at once.
tefb7a0f9 = Діє одразу.

# Your browser
tb5a48dd2 = Ваш браузер

# Its private key is in the system keychain.
t62a47808 = Його закритий ключ зберігається в системній зв’язці ключів.

# Its private key is in ca.key, which only you can read.
td9c6165d = Його закритий ключ зберігається в ca.key, який можете читати лише ви.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = Його закритий ключ зберігається в ca.key, який можете читати лише ви, бо системна зв’язка ключів не може його прийняти: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Відкрийте сторінку. Якщо браузер попереджає про сертифікат або не може знайти сторінку, один із кроків вище для цього браузера не виконано.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } працює через tracewing і довіряє йому.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } працює через tracewing, але відкрив сторінку без HTTPS. Відкрийте її знову через https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = Телефон або інший комп’ютер також може надсилати сюди свій трафік. Увімкніть «Дозволити іншим пристроям використовувати проксі» в Налаштуваннях.

# this computer's address, port {port}
t2188b3f4 = адреса цього комп’ютера, порт { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = На пристрої вкажіть проксі Wi-Fi { $addr }. Потім відкрийте там http://{ $host }, отримайте сертифікат і довіртеся йому. Потоки пристрою показують його адресу, і їх можна фільтрувати за допомогою device:. Дайте пристрою назву з будь-якого з них на вкладці «З’єднання».

# Looking for it…
t50ee7452 = Пошук…

# Installing. The system may ask you to confirm.
ta41c38ec = Встановлення. Система може попросити підтвердження.

# Removing. The system may ask you to confirm.
t62b22726 = Видалення. Система може попросити підтвердження.

# This computer trusts it.
t27d6bacc = Цей комп’ютер довіряє йому.

# Installed in: {places}.
tb5bcc244 = Встановлено в: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Інструменти командного рядка довіряють йому, але браузери мають власні сховища. Доки його там немає, HTTPS через tracewing у браузері не працює.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing читає HTTPS за допомогою сертифіката, створеного лише для цього комп’ютера. Доки йому не довіряють, HTTPS через tracewing не працює.

# Trust the certificate
t26618699 = Довіряти сертифікату

# Install
tb50cbd40 = Встановити

# Also for command-line tools
t0db8faea = Також для інструментів командного рядка

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Видалити сертифікат з усіх сховищ цього комп’ютера? HTTPS через tracewing не працюватиме, доки його не встановлять знову. tracewing збереже його, щоб установити пізніше.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = У Linux браузерам потрібен certutil, щоб їм можна було сказати довіряти сертифікату. Установіть його, потім натисніть тут «Встановити»:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = У Linux браузерам потрібен certutil, щоб їм можна було сказати довіряти сертифікату. Установіть NSS-інструменти свого дистрибутива, потім натисніть тут «Встановити».

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = У цьому середовищі робочого столу немає системного проксі, який tracewing може змінити. Відкрийте нижче браузер окремо в розділі «Браузери» або термінал.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = У { $desktop } немає системного проксі, який tracewing може змінити. Відкрийте нижче браузер окремо в розділі «Браузери» або термінал.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Застосунки, що використовують системне налаштування, тепер працюють через { $addr }. Воно повернеться до попереднього, коли tracewing закриється або аварійно завершить роботу.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Спрямовує трафік цього комп’ютера через tracewing для застосунків, що використовують системне налаштування. Або самі вкажіть { $addr } в окремому застосунку.

# Turn on the system proxy
t52b88441 = Увімкнути системний проксі

# Turn off
tcd03e04b = Вимкнути

# Turn on
t65a0c5a7 = Увімкнути

# Check it in your browser
tbed6e83a = Перевірте в браузері

# Open the test page
te4e36f65 = Відкрити тестову сторінку

# Set up
t43c06abc = Підготовка

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Три кроки, щоб бачити HTTPS-трафік цього комп’ютера.

# See traffic
t980eee79 = Переглянути трафік

# the browsers' own stores
tddf6836c = власні сховища браузерів

# Trusts the certificate.
tc5bba858 = Довіряє сертифікату.

# Does not trust the certificate yet.
tf2a0b8c5 = Поки не довіряє сертифікату.

# Looking…
t284579ba = Пошук…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Профілю ще немає. Відкрийте браузер один раз, потім пошукайте знову.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 =
    { $n ->
        [one] { $holding } з { $n } профілю довіряють сертифікату.
        [few] { $holding } з { $n } профілів довіряють сертифікату.
        [many] { $holding } з { $n } профілів довіряють сертифікату.
       *[other] { $holding } з { $n } профілю довіряють сертифікату.
    }

# Has not opened the test page yet.
t67f9cb49 = Тестову сторінку ще не відкривав.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Відкрив тестову сторінку через tracewing, по HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Відкрив тестову сторінку через tracewing, але без HTTPS.

# Browsers
td580efd4 = Браузери

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Кожен браузер сам вирішує, чому довіряти й куди надсилати трафік. «Перевірити» відкриває в ньому тестову сторінку. «Відкрити окремо» запускає його з власним профілем, що йде через tracewing, не чіпаючи системний проксі.

# Look again
t98416222 = Пошукати знову

# No browser found on this computer.
t0e46bb34 = На цьому комп’ютері браузерів не знайдено.

# Check
t592c8de7 = Перевірити

# Open on its own
t0da7783c = Відкрити окремо

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = UDP-порт 443 на цьому комп’ютері заблоковано, тож застосунки переходять з HTTP/3 на HTTP/2 і проходять через tracewing. Блокування припиниться, коли tracewing закриється або аварійно завершить роботу.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Деякі застосунки використовують HTTP/3 поверх UDP і обходять проксі. Якщо заблокувати UDP-порт 443 на цьому комп’ютері, вони перейдуть на HTTP/2 через tracewing. Потрібен пароль адміністратора; блокування припиняється, коли tracewing закривається. Тим часом VPN, що використовує UDP-порт 443, не працюватиме.

# Apps on HTTP/3
t2cfe5270 = Застосунки на HTTP/3

# Stop blocking
t318358d8 = Припинити блокування

# Block HTTP/3
t88231e40 = Заблокувати HTTP/3

# Or one app at a time
t91c63f1c = Або по одному застосунку

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Термінал, що йде через tracewing, а системний проксі та інші застосунки лишаються як є. Для браузера те саме робить «Відкрити окремо» в розділі «Браузери», з власним профілем.

# Open a terminal
t4b629c72 = Відкрити термінал

# iPhone and iPad
ta550cc97 = iPhone та iPad

# Flutter apps
ta04ee397 = Застосунки Flutter

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. У Параметрах → Wi-Fi виберіть мережу, потім «Налаштувати проксі» → «Вручну»: сервер { $server }, порт { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Відскануйте код камерою або відкрийте { $page } у Safari й отримайте сертифікат. Дозвольте завантаження.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. У Параметрах → Загальні → «VPN і керування пристроями» виберіть профіль tracewing CA й установіть його.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. У Параметрах → Загальні → «Про пристрій» → «Параметри довіри сертифікатам» увімкніть повну довіру для tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Відкрийте https://{ $host } у Safari. Якщо там написано, що все працює, застосунки, які використовують системні налаштування, проходять через tracewing.

# When you are done
t62466f08 = Коли закінчите

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. У Параметрах, Wi-Fi виберіть мережу, потім Налаштування проксі: Вимк. Якщо залишити, пристрій лишиться без інтернету в цій мережі, коли tracewing закриється.

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. У Параметрах, Загальні, VPN і керування пристроєм виберіть tracewing CA і видаліть профіль. Повна довіра піде разом із ним.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. У Налаштуваннях → Wi-Fi змініть мережу й у розділі «Додатково» встановіть для проксі «Вручну»: хост { $host }, порт { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Відскануйте код або відкрийте { $page } у браузері й отримайте сертифікат.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. У Налаштуваннях → Безпека → «Шифрування й облікові дані» (назви залежать від виробника) виберіть «Установити сертифікат», потім «Сертифікат ЦС», і виберіть tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Починаючи з Android 7, застосунки довіряють сертифікатам, доданим користувачем, лише якщо це в них указано. Для налагоджувальної збірки власного застосунку додайте res/xml/network_security_config.xml:

# and point to it from AndroidManifest.xml:
t02ad844b = і пошліться на нього в AndroidManifest.xml:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Інші застосунки й далі відхилятимуть сертифікат: їхній трафік відображається як невдалі потоки.

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. У Налаштуваннях, Wi-Fi змініть мережу й поверніть проксі на Немає. Якщо залишити, пристрій лишиться без інтернету в цій мережі, коли tracewing закриється.

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. У Налаштуваннях, Безпека, Шифрування й облікові дані, Облікові дані користувача виберіть tracewing і видаліть. Видаліть і tracewing.crt із Завантажень.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = HttpClient у Dart, який використовують http і Dio у Flutter, не враховує налаштування проксі телефона, а на Android не довіряє сертифікатам, доданим користувачем. У налагоджувальній збірці спрямуйте його сюди:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback пропускає будь-який сертифікат, тому він лише для налагоджувальних збірок.

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = Приберіть TracewingProxy і його рядок у main() перед випуском, хоча поза налагоджувальними збірками вони нічого не роблять. На телефоні нічого не змінювалося.

# Open settings
t1336b4d6 = Відкрити налаштування

# Phones and other devices
t2e3c591c = Телефони та інші пристрої

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Телефон, планшет або інший комп’ютер у цій мережі може працювати через tracewing за адресою { $proxy }. Код відкриває сторінку з його сертифікатом. Потоки пристрою показують його адресу, їх можна фільтрувати за допомогою device:, а сам пристрій можна назвати з будь-якого з них на вкладці «З’єднання».

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = Інші пристрої не можуть під’єднатися до tracewing за однією з адрес цього комп’ютера: { $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f =
    { $n ->
        [one] { $n } пристрій пройшов через tracewing ({ $names }). Коли закінчите, вимкніть на ньому проксі Wi-Fi, як описано нижче: якщо залишити, він лишиться без інтернету в цій мережі, коли tracewing закриється.
        [few] { $n } пристрої пройшли через tracewing ({ $names }). Коли закінчите, вимкніть на них проксі Wi-Fi, як описано нижче: якщо залишити, вони лишаться без інтернету в цій мережі, коли tracewing закриється.
        [many] { $n } пристроїв пройшли через tracewing ({ $names }). Коли закінчите, вимкніть на них проксі Wi-Fi, як описано нижче: якщо залишити, вони лишаться без інтернету в цій мережі, коли tracewing закриється.
       *[other] { $n } пристрою пройшли через tracewing ({ $names }). Коли закінчите, вимкніть на них проксі Wi-Fi, як описано нижче: якщо залишити, вони лишаться без інтернету в цій мережі, коли tracewing закриється.
    }

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Налаштовано. Сертифікат перебуває серед системних до перезапуску пристрою.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Налаштовано. Сертифікат у папці «Завантаження»: у щойно відкритих Налаштуваннях виберіть «Шифрування й облікові дані» → «Установити сертифікат» → «Сертифікат ЦС» і виберіть tracewing.crt.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Застосунки, які довіряють лише системним сертифікатам, однаково його відхилятимуть.

# An emulator.
t14b6b86d = Емулятор.

# On USB.
t67dafc14 = Через USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Прийміть запит на налагодження через USB на пристрої, а потім пошукайте знову.

# Still starting. Look again in a moment.
t13f68b64 = Ще запускається. Пошукайте знову за мить.

# adb says it is {state}.
t2b7d71da = adb повідомляє стан: { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Налаштовує проксі на емуляторі Android або телефоні, підключеному через USB з увімкненим налагодженням через USB, і встановлює сертифікат. Трафік іде через ADB, тому жодному з них не потрібно бути в цій мережі.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = adb не знайдено. Установіть Android Studio або Android Platform Tools, а потім пошукайте знову.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Жоден пристрій не підключено. Запустіть емулятор або підключіть телефон з увімкненим налагодженням через USB, а потім пошукайте знову.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Налаштуйте пристрій, щоб він надсилав сюди свій трафік. Відміна або закриття tracewing знову прибирає з нього проксі.

# Android emulators and USB devices
t379e433c = Емулятори Android і USB-пристрої

# Find devices
t54d4af39 = Знайти пристрої

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = Системний проксі й блокування HTTP/3 повертаються як було, коли tracewing закривається або падає, як і Android-пристрій, налаштований через USB. Сертифікат лишається довіреним, доки «Видалити» на кроці 1 його не прибере. Телефон, налаштований вручну, зберігає проксі Wi-Fi і сертифікат, доки їх не приберуть на ньому: як це зробити, сказано в інструкції для кожного пристрою. Інші пристрої можуть під’єднуватися до проксі цього комп’ютера, доки це не вимкнено в Налаштуваннях.

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code та інші інструменти ШІ можуть переглядати й читати потоки, які записує tracewing, через його MCP-сервер. Токени й cookie не передаються, якщо Налаштування цього не дозволяють. У Claude Code:

# Elsewhere, in the tool's MCP settings:
tec38833c = Деінде — у налаштуваннях MCP інструмента:

# Flows
t4db77db2 = Потоки

# Choose the flow above or below
tecb3adec = Вибрати потік вище або нижче

# The first or the last flow
t7e278a00 = Перший або останній потік

# A page up or down
t23b1814b = На сторінку вгору або вниз

# Filter
tf4a9c097 = Фільтр

# Pause or resume recording
t6e21519a = Призупинити або відновити запис

# Clear the list
tf2a55d1f = Очистити список

# Compare the flow clicked with the chosen one
te42cd067 = Порівняти натиснутий потік із вибраним

# The chosen flow
te0a729de = Вибраний потік

# Its request
tb3970784 = Його запит

# Its response
t221fc168 = Його відповідь

# Send it again
tef7d1944 = Надіслати його знову

# Edit it and send it
t6bc99e5c = Змінити й надіслати

# Copy it as cURL
tc99a11a3 = Копіювати як cURL

# The part before or after the one shown
tdf4221d9 = Розділ до або після показаного

# Save the body shown
t20d9b7c0 = Зберегти показане тіло

# Select all of the part shown
tf9168168 = Виділити все в показаній частині

# Copy the selected text
t00b090d3 = Копіювати виділений текст

# Send it on, as edited
t8c35fa5a = Надіслати з правками

# Send it on as it was
t16f7abb5 = Надіслати як було

# Drop it
t4e7f48b9 = Відкинути

# The window
t2af18488 = Вікно

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Підготовка, Трафік, Правила, Призупинені, Створити запит, Сеанси, Налаштування

# Show or hide the sidebar
t67adca04 = Показати або сховати бічну панель

# Show or hide the inspector
ta5a7b9e9 = Показати або сховати інспектор

# The inspector beside the list or below it
t51e214f7 = Інспектор поруч зі списком або під ним

# The next or the previous field
ta4b4bd5e = Наступне або попереднє поле

# These shortcuts
tbdf3b62b = Ці комбінації клавіш

# Close them
tf3a4b1ef = Закрити їх

# Keyboard shortcuts
t31eaddb7 = Комбінації клавіш

# Close
tcd86acc3 = Закрити

# The signature in this body now works out to {now}, not {was}: it is no longer the transaction that was signed. tracewing holds no keys, so it cannot sign the new one.
td0d9c66b = Підпис у цьому тілі тепер дає { $now }, а не { $was }: це вже не та транзакція, яку підписали. tracewing не має ключів, тож нову підписати не може.

# What this body carries no longer reads as a signed transaction. tracewing holds no keys, so it cannot sign it again.
td97ae94b = Те, що несе це тіло, більше не читається як підписана транзакція. tracewing не має ключів, тож підписати її знову не може.

# This body carries a signed transaction and its bytes have changed. The signatures are over the bytes as they were, and tracewing can neither check them nor make new ones.
ta6e69f9a = Це тіло несе підписану транзакцію, і його байти змінилися. Підписи стоять під байтами, якими вони були, і tracewing не може ні перевірити їх, ні створити нові.

# The certificate: {e}
t0235648c = Сертифікат: { $e }

# The terminal
tf542b2f4 = Термінал

# The system proxy: {e}
tc99726ce = Системний проксі: { $e }

# Blocking HTTP/3: {e}
t024ec871 = Блокування HTTP/3: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx або текст у тілах

# A name, as iPhone test
t19f89214 = Назва, наприклад Тестовий iPhone

# A name, as Phone
tfce439bf = Назва, наприклад Phone

# A message to send; {keys} sends it
tf9d9c09b = Повідомлення для надсилання; { $keys } — надіслати

# The search index: {e}
te3c971c9 = Пошуковий індекс: { $e }

# Click the flow to compare #{id} with.
tac1ec1d2 = Натисніть на потік, щоб порівняти його з #{ $id }.

# An earlier session: {name}
t73446a1d = Попередній сеанс: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Лише читання. Пошук охоплює більше в міру побудови індексу. Запис триває.

# Read-only. Recording goes on.
te2f35d9e = Лише читання. Запис триває.

# Back to live
t317c0453 = До живого трафіку

# {shown} of {total}
tf2cc0563 = { $shown } з { $total }

# Recording
te75cf20c = Запис

# System proxy off · {addr}
t7e813d25 = Системний проксі вимкнено · { $addr }

# System proxy…
t29e7dcd0 = Системний проксі…

# System proxy on · {addr}
t319ad6b9 = Системний проксі ввімкнено · { $addr }

# Apps
t53d91aad = Застосунки

# Devices
t27acf320 = Пристрої

# All hosts
tfc7bfe37 = Усі хости

# No host has this in its name
t838c1d44 = Немає хостів із цим в імені

# Hidden
t3c2dfe59 = Приховані

# Hosts
tcf9aefb4 = Хости

# Newest
t324b6d0b = Найновіші

# No flow fits this filter
t7eb295f4 = Жоден потік не відповідає цьому фільтру

# Nothing yet
tbd00747c = Поки нічого

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Увімкніть системний проксі вище або спрямуйте застосунок на { $addr }. Щоб спробувати: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Поруч зі списком

# Below the list
t23658daf = Під списком

# Drop it on a place · Esc puts it back
t0fa632ba = Перетягніть на місце · Esc повертає назад

# Traffic
t4b589266 = Трафік

# Certificate trusted on this computer
t7b93f551 = Цей комп’ютер довіряє сертифікату

# Checking the certificate…
tbf8942c3 = Перевірка сертифіката…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Встановлення сертифіката; система може попросити підтвердження…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Видалення сертифіката; система може попросити підтвердження…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS не працюватиме, доки сертифікат не стане довіреним: налаштувати

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml не використано: { $problem }

# A setting was not saved: {problem}
tff1a5018 = Налаштування не збережено: { $problem }

# The proxy stopped: {failure}
t04dcff11 = Проксі зупинився: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = Диск майже заповнений: тіла не зберігаються

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
        [one] { $count } потік не записано
        [few] { $count } потоки не записано
        [many] { $count } потоків не записано
       *[other] { $count } потоку не записано
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
        [one] { $count } потік без тіл
        [few] { $count } потоки без тіл
        [many] { $count } потоків без тіл
       *[other] { $count } потоку без тіл
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
        [one] Видалено { $count } найстаріший потік, щоб не перевищити { $size }
        [few] Видалено { $count } найстаріші потоки, щоб не перевищити { $size }
        [many] Видалено { $count } найстаріших потоків, щоб не перевищити { $size }
       *[other] Видалено { $count } найстарішого потоку, щоб не перевищити { $size }
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
        [one] { $count } потік
        [few] { $count } потоки
        [many] { $count } потоків
       *[other] { $count } потоку
    }

# Export as HAR
t95b6b766 = Експортувати як HAR

# Shortcuts {keys}
t4ab0ecb4 = Комбінації клавіш { $keys }

# Columns
tea156fdc = Стовпці

# base64url
tadfc8ac3 = base64url

# base64
tf04e6eba = base64

# Hex
t613f34ea = Hex

# Percent-encoded
t7c7bb5a9 = Відсоткове кодування

# Escaped text
t195b3db9 = Текст з escape-послідовностями

# HTML entities
t54958ecb = HTML-сутності

# Version
t5dcdd537 = Версія

# Variant
t8433b026 = Варіант

# UUID
t257b8832 = UUID

# ULID
tf19e9db9 = ULID

# ObjectId
t427abd77 = ObjectId

# Counter
tfc0f33c3 = Лічильник

# KSUID
te30e0b47 = KSUID

# Worker
tde5c1a77 = Воркер

# Sequence
t64cec188 = Послідовність

# Snowflake
te00a2b87 = Snowflake

# JSON, {n} fields
tc6a0fbd2 = JSON, полів: { $n }

# JSON, {n} items
t06f5832d = JSON, елементів: { $n }

# JSON in a string
tc69f7572 = JSON у рядку

# Expires
tc7aa0523 = Спливає

# Issued
tb1835ecc = Видано

# Not before
td10dacd9 = Не раніше

# Algorithm
t9945210a = Алгоритм

# State
t1deca496 = Стан

# expired
t338eb632 = прострочений

# JWT
t6076261e = JWT

# {n} row | {n} rows
td8d653d6 =
    { $n ->
        [one] { $n } рядок
        [few] { $n } рядки
        [many] { $n } рядків
       *[other] { $n } рядка
    }

# In hex and binary
tc31ef85f = У hex і двійковому вигляді

# Bits set
tb67e8e09 = Встановлені біти

# As a size
ta9175b27 = Як розмір

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
tfda57c3e = через { $amount }

# {amount} ago
t5f968014 = { $amount } тому

# Unix seconds
t6edf5796 = Unix-секунди

# Unix milliseconds
t0d28f4b7 = Unix-мілісекунди

# Unix microseconds
tc705fe24 = Unix-мікросекунди

# Unix nanoseconds
t235cb430 = Unix-наносекунди

# ISO 8601
t402bdff3 = ISO 8601

# ISO 8601, no zone: read as UTC
t3bb6c7ca = ISO 8601, без зони: прочитано як UTC

# HTTP-date
te55df34c = HTTP-date

# Seconds
t7e12f48a = Секунди

# Trace context
ta58c7c95 = Контекст трасування

# Trace
t26f8e74e = Трасування

# Span
taba837e1 = Спан

# Sampled
t7e4a1d99 = Зібрано

# yes
t4e9f3590 = так

# no
t62366ffa = ні

# Parent span
t3cf3f9c3 = Батьківський спан

# B3 trace
tb4f9b9bb = Трасування B3

# Began
t3a1b8566 = Початок

# Amazon trace
t122e82a4 = Трасування Amazon

# Server-Timing
tb79773df = Server-Timing

# Took
tcea1562e = Тривалість

# About
t58e096de = Опис

# Scheme
t9c9083a2 = Схема

# Port
te2c53cc6 = Порт

# User
te0a63e12 = Користувач

# {n} fields
t98976d50 = полів: { $n }

# Fragment
t7889e7fd = Фрагмент

# Query fields
t2f58f1e0 = Поля запиту

# Field
t0fb83d07 = Поле

# Value
td147f96a = Значення

# Basic authentication
te161b66b = Аутентифікація Basic

# Password
t2cc30838 = Пароль

# never kept
t83c57deb = ніколи не зберігається

# kept {span}
t01f67b47 = зберігається { $span }

# kept {span} by shared caches
t5af054c4 = у спільних кешах зберігається { $span }

# by the browser only
tfb7516b7 = лише браузером

# by anyone
t86fe63fe = будь-ким

# checked every time
t02076cba = перевіряється щоразу

# not used once stale
t0f7f5bf9 = не використовується, коли застарів

# never checked again
tc01144d0 = більше не перевіряється

# Cache-Control
t3868d34d = Cache-Control

# Status code
t20b704bc = Код статусу

# Cookie
tbdbabadf = Cookie

# data: URI, {media}
t514d8ddb = data:-URI, { $media }
