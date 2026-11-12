# tracewing in Russian. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 =
    { $n ->
        [one] … { $count } строка { $closer }
        [few] … { $count } строки { $closer }
        [many] … { $count } строк { $closer }
       *[other] … { $count } строки { $closer }
    }

# … {size} more
ta8dc6eab = … ещё { $size }

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc =
    { $n ->
        [one] { $size } { $format } · { $count } строка JSON
        [few] { $size } { $format } · { $count } строки JSON
        [many] { $size } { $format } · { $count } строк JSON
       *[other] { $size } { $format } · { $count } строки JSON
    }

# The first {count} rows are shown.
t16aa1c6d = Показаны только первые { $count } строк.

# Unfold all
t3849d44e = Развернуть всё

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } строка
        [few] { $size } · { $count } строки
        [many] { $size } · { $count } строк
       *[other] { $size } · { $count } строки
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · чтение { $percent }%

# Appearance: {now}. Click for {next}.
t332cfa6d = Оформление: { $now }. Нажмите, чтобы переключить: { $next }.

# Method
t98a2c750 = Метод

# Status
t005ef20f = Статус

# App
t81f51acc = Приложение

# Host
t302af18f = Хост

# Path
teb66e456 = Путь

# Operation
t218e02a6 = Операция

# Size
ta6478e7c = Размер

# Time
tdfe4e404 = Время

# Number
t61e9eac0 = Номер

# A body was not kept, so the two cannot be compared.
t9d6abb2a = Одно из тел не сохранено, поэтому сравнить их нельзя.

# {count} field differs | {count} fields differ
t6a8fff7f =
    { $n ->
        [one] Отличается { $count } поле
        [few] Отличаются { $count } поля
        [many] Отличаются { $count } полей
       *[other] Отличается { $count } поля
    }

# … {count} line the same | … {count} lines the same
tcc37e7ea =
    { $n ->
        [one] … { $count } одинаковая строка
        [few] … { $count } одинаковые строки
        [many] … { $count } одинаковых строк
       *[other] … { $count } одинаковой строки
    }

# Nothing to compare.
t9f32d942 = Нечего сравнивать.

# URL
t7569633e = URL

# Protocol
t4a57ebd5 = Протокол

# Type
td155d06d = Тип

# Sizes
td1a4a19d = Размеры

# Body
t428a1095 = Тело

# Headers
t334f5989 = Заголовки

# Query
t9e89e783 = Запрос

# Request
t82454152 = Запрос

# Response
t55a982fe = Ответ

# Neither has headers kept.
tb43ef094 = Ни у одного нет сохранённых заголовков.

# Trailers
t7c44fa89 = Трейлеры

# Neither URL has a query.
t4d8203e0 = Ни в одном URL нет query.

# Comparing…
ta8565d31 = Сравнение…

# No body
tfa4c2902 = Без тела

# {kind}, its first {size} compared
t1d894c39 = { $kind }, сравнены первые { $size }

# Neither has a body.
tf2c47725 = Ни у одного нет тела.

# The bodies are the same.
t85152e01 = Тела одинаковые.

# {count} field differs. | {count} fields differ.
t0f651f77 =
    { $n ->
        [one] Отличается { $count } поле.
        [few] Отличаются { $count } поля.
        [many] Отличаются { $count } полей.
       *[other] Отличается { $count } поля.
    }

# The first {count} are listed.
taae62a3e = Показаны только первые { $count }.

# {count} line differs. | {count} lines differ.
tf57a3157 =
    { $n ->
        [one] Отличается { $count } строка.
        [few] Отличаются { $count } строки.
        [many] Отличаются { $count } строк.
       *[other] Отличается { $count } строки.
    }

# Fields
t3308bd9c = Поля

# Lines
t8a1a00dc = Строки

# Compare
td57a0a54 = Сравнение

# #{a} beside #{b}: what only #{a} has is red, what only #{b} has is green.
t71ce2de9 = #{ $a } рядом с #{ $b }: то, что есть только в #{ $a }, — красным, то, что есть только в #{ $b }, — зелёным.

# Swap
te519d36e = Поменять местами

# Back to Traffic
t644042d2 = Назад в Traffic

# "{method}" is not a method
tacdcf5cb = "{ $method }" — не метод

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = тело запроса сохранено не целиком, поэтому его нельзя отправить повторно

# name: value, one a line
t4df37bea = имя: значение, по одному на строку

# No flow is chosen
t550b0a98 = Поток не выбран

# This flow: {e}
t296762f8 = Этот поток: { $e }

# The body is not a {message}: {e}
t3220a10a = Тело не является { $message }: { $e }

# Protobuf {message}, written as JSON: it is sent as Protobuf.
t83b6cd6e = Protobuf { $message }, записанный как JSON: отправляется как Protobuf.

# Sending…
te946e7bf = Отправка…

# Send
tf28e14cf = Отправить

# New
t8b242db1 = Новый

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … ещё { $count } байт: в потоке в разделе Трафик есть всё.

# Body · {message}
taa3cc3ff = Тело · { $message }

# Compose
t18049f93 = Конструктор

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Ваш собственный запрос или отправленный повторно. Он идёт через tracewing, поэтому к нему применяются правила, и он тоже виден в разделе Трафик.

# {version} both sides
t961a5ea0 = { $version } с обеих сторон

# {client} to the proxy, {server} to the server
t3d237646 = { $client } до прокси, { $server } до сервера

# Queued
t9e437c74 = В очереди

# Connect
tdb89a899 = Подключение

# Waiting
tc9450e48 = Ожидание

# Download
t6964bb19 = Загрузка

# No file picker: {e}
t6935fbd3 = Нет окна выбора файла: { $e }

# Writing the HAR…
t725a0e89 = Запись HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
        [one] { $n } поток записан в { $path }
        [few] { $n } потока записано в { $path }
        [many] { $n } потоков записано в { $path }
       *[other] { $n } потока записано в { $path }
    }

# The HAR was not written: {e}
tef8ec569 = HAR не записан: { $e }

# Open
t538b10e9 = Открыть

# Reading {path}…
tc0e7c8f4 = Чтение { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
        [one] , { $n } запись пропущена
        [few] , { $n } записи пропущено
        [many] , { $n } записей пропущено
       *[other] , { $n } записи пропущено
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
        [one] { $n } поток прочитан ({ $format })
        [few] { $n } потока прочитано ({ $format })
        [many] { $n } потоков прочитано ({ $format })
       *[other] { $n } потока прочитано ({ $format })
    }

# The file was not read: {e}
t7f49510f = Файл не прочитан: { $e }

# it is no longer kept
ta7b4c380 = больше не хранится

# {size} saved to {path}
tc6932467 = { $size } сохранено в { $path }

# The body was not saved: {e}
t189900b4 = Тело не сохранено: { $e }

# Fiddler session
t9f5c89a3 = Сессия Fiddler

# Charles session
tb4f56f2b = Сессия Charles

# All
t75afe364 = Все

# Errors
te9ed60a6 = Ошибки

# Media
td10fcaa3 = Медиа

# Rule: {rule}
teca2074f = Правило: { $rule }

# Events
t003420a4 = События

# JSON lines
te8ce2db0 = Строки JSON

# Not decrypted
t8090a1ba = Не расшифровано

# open
td35ec4c9 = открыто

# Reading the request…
t51c96267 = Чтение запроса…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = Тело запроса не сохранено, поэтому его запрос нельзя показать.

# An operation without a name
t38c4d30b = Операция без имени

# {n} of {count}: {name}
t53045460 = { $n } из { $count }: { $name }

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Persisted query: текст хранит сервер, а запрос отправляет только его хеш.

# Variables
t7996f062 = Переменные

# Cut
t58e25863 = Вырезать

# Copy
t658f3664 = Копировать

# Paste
tb34a0288 = Вставить

# Select all
t6af21072 = Выделить всё

# This flow is gone.
t3343999e = Этого потока больше нет.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Этот хост не расшифровывается, поэтому то, что он отправляет, не видно.

# Still coming: it shows once it ends.
t42be95fb = Ещё идёт: появится, когда закончится.

# No body.
t31e4d044 = Без тела.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Это тело не сохранено: диск был почти заполнен или в очереди было слишком много данных.

# This body was not kept.
te46616b5 = Это тело не сохранено.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Этого тела больше нет: оно удалено вместе с самыми старыми потоками или не читается.

# Cannot open this body: {e}
t620a039b = Не удаётся открыть это тело: { $e }

# Inspector
t4def410e = Инспектор

# Choose a flow to see it here
tc7be91e8 = Выберите поток, чтобы увидеть его здесь

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Не расшифровано. Новые подключения к этому хосту проходят без чтения.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Если приложение не работает с этим хостом, возможно, оно использует пиннинг сертификатов.

# Make a rule
t0ac0e062 = Создать правило

# Decrypt this host
tb7367e2c = Расшифровывать этот хост

# Do not decrypt this host
t5fa113f6 = Не расшифровывать этот хост

# Request and response
t04e060e6 = Запрос и ответ

# Request and response one at a time
t70397119 = Запрос и ответ по очереди

# Request and response side by side
t3d35ce69 = Запрос и ответ рядом

# Request above the response
t83c63123 = Запрос над ответом

# Table
td70ceb7f = Таблица

# Pretty
t7e09470d = Формат

# Raw
ta689aa91 = Исходный

# Picture · {size}
te97ed633 = Изображение · { $size }

# Messages {count}
te61b841a = Сообщения { $count }

# Headers {headers}
t04b0146d = Заголовки { $headers }

# Cookies {cookies}
td58eb84b = Cookie { $cookies }

# Query {query}
t4756f9f5 = Параметры { $query }

# Form {form}
tfa6e7203 = Форма { $form }

# Connection
t09ea4ff9 = Подключение

# Save body
te2c44424 = Сохранить тело

# Send again
tf5910557 = Отправить повторно

# Edit and send
tf837e542 = Изменить и отправить

# Copied
t8dc21305 = Скопировано

# Copy as cURL
t866d1e3e = Копировать как cURL

# Click another flow…
t59253578 = Нажмите на другой поток…

# Compare with…
t58dfe040 = Сравнить с…

# Compare with the call before
tef15286c = Сравнить с предыдущим вызовом

# Copy as…
t9c1ad448 = Копировать как…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Это изображение не удаётся отобразить. Сохраните тело, чтобы посмотреть его.

# Opening…
tc40f2321 = Открытие…

# No response
t8c2c37b1 = Нет ответа

# Tunnel, not decrypted
tce693254 = Туннель, не расшифровано

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# RPC: {method}
tf8162891 = RPC: { $method }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Через подключение { $id }, созданное ранее: на подключение время не ушло.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Не найдено: возможно, процесс завершился или принадлежит другому пользователю

# On another device
t2b964a37 = На другом устройстве

# CLIENT
t6d3cd99e = КЛИЕНТ

# Device
tbdcbb413 = Устройство

# Program
tc4253b6b = Программа

# Address
t700560f3 = Адрес

# SERVER
te3b27612 = СЕРВЕР

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = Сведения об этом подключении больше не хранятся: они удалены вместе с более старыми потоками.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Не расшифровано: tracewing передал его дальше, не заглядывая внутрь.

# No connection was made.
tf348eb92 = Подключение не устанавливалось.

# None: it was answered without one.
tac71808e = Нет: ответ дан без подключения.

# {id}, given up during TLS
t0ec66a8c = { $id }, прервано во время TLS

# {id}, made for this flow
t7fe32cfc = { $id }, создано для этого потока

# {id}, made before and used again
ta904c078 = { $id }, создано ранее и использовано повторно

# Server
t6d7302f2 = Сервер

# None: plain HTTP
t05606349 = Нет: обычный HTTP

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = СЕРВЕР НЕ ПОКАЗАЛ СЕРТИФИКАТОВ

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = ЧТО ПОКАЗАЛ СЕРВЕР, СНАЧАЛА ЕГО СОБСТВЕННЫЙ

# NO CERTIFICATES
te6f31dce = НЕТ СЕРТИФИКАТОВ

# CERTIFICATE
t1c902790 = СЕРТИФИКАТ

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
        [one] { $n } СЕРТИФИКАТ, СНАЧАЛА СЕРВЕРНЫЙ
        [few] { $n } СЕРТИФИКАТА, СНАЧАЛА СЕРВЕРНЫЙ
        [many] { $n } СЕРТИФИКАТОВ, СНАЧАЛА СЕРВЕРНЫЙ
       *[other] { $n } СЕРТИФИКАТА, СНАЧАЛА СЕРВЕРНЫЙ
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing не стал продолжать с этим сервером: { $error }. Если приложение доверяет ему иным способом, например через корпоративный CA, выберите «Не расшифровывать этот хост», и приложение само проверит сервер.

# Cipher
t5b3bed6a = Шифр

# none
tada7afdb = нет

# not valid until {when}
t0a14b2d8 = начнёт действовать { $when }

# expired {when}
t932538c2 = истёк { $when }

# expires {when}
t679bcb51 = истекает { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
        [one] { $names } и ещё { $n }
        [few] { $names } и ещё { $n }
        [many] { $names } и ещё { $n }
       *[other] { $names } и ещё { $n }
    }

# Issued by {issuer}
t15a1ce5a = Выдан: { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Действителен с { $from } по { $to } · { $when }

# Names {names}
tda75e459 = Имена { $names }

# Serial {serial}
td1f0d98b = Серийный номер { $serial }

# These headers were not kept.
tb8739fec = Эти заголовки не сохранены.

# No headers.
tf3dacb5e = Нет заголовков.

# Trailers, sent after the body
t3447e9de = Трейлеры, отправленные после тела

# File {filename} · {kind} · {size}
t6dbd239f = Файл { $filename } · { $kind } · { $size }

# no type
t902a6ece = без типа

# No cookies were sent.
t58e85ab2 = Cookie не отправлялись.

# No cookies were set.
tf57dbabc = Cookie не устанавливались.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · подпись не проверена

# JWT · signature not checked
t05f8b533 = JWT · подпись не проверена

# Header
t114f9980 = Заголовок

# Payload
ta0639745 = Payload

# Expires
tc7aa0523 = Истекает

# Issued
tb1835ecc = Выдан

# Not before
td10dacd9 = Не ранее

# in {amount}
tfda57c3e = через { $amount }

# {amount} ago
t5f968014 = { $amount } назад

# Light
t8ad2f8cf = Светлое

# Dark
t45570065 = Тёмное

# System
t8f3184bc = Системное

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
        [one] передано частями: { $count }
        [few] передано частями: { $count }
        [many] передано частями: { $count }
       *[other] передано частями: { $count }
    }

# stopped: {stop}
tfa48fb30 = остановлено: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = токены: { $input } на входе · { $output } на выходе

# Reading the call…
t33b52a3c = Чтение вызова…

# Answer
tf94ea0ff = Ответ

# Thinking: {thinking}
t737ece33 = Размышления: { $thinking }

# Calls {name}{id}
t017882e4 = Вызывает { $name }{ $id }

# Nothing yet.
td9b71d16 = Пока ничего.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
        [one] { $text }… и ещё { $count } символ
        [few] { $text }… и ещё { $count } символа
        [many] { $text }… и ещё { $count } символов
       *[other] { $text }… и ещё { $count } символа
    }

# Copy the answer
t69975ed7 = Копировать ответ

# Asked
t4e3af9c3 = Вопрос

# Tools offered: {tools}
tb871bcf4 = Предложенные инструменты: { $tools }

# Not kept
tba30b5fa = Не сохранено

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Это сообщение не сохранено: в очереди было слишком много данных или диск был почти заполнен.

# This message is empty.
tdb0ec01f = Это сообщение пустое.

# No messages yet
t51e7624c = Сообщений пока нет

# {said} kept, of {size}
t3558745a = Сохранено { $said } из { $size }

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } — не целое число

# Latency
t23a45ef9 = Задержка

# Down
ta4995035 = Загрузка

# Up
t42f35940 = Выгрузка

# Loss
t69db303a = Потери

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Укажите задержку, скорость или потери, чтобы замедлить сеть

# Latency, ms
tefe18ab9 = Задержка, ms

# Down, kbit/s
t08c4ae33 = Загрузка, kbit/s

# Up, kbit/s
td55cfe9a = Выгрузка, kbit/s

# Loss, %
tea4af439 = Потери, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Только эти хосты, например *.example.com; если пусто — все

# Not kept for next time: {e}
tdce4d67c = Не сохранено на следующий раз: { $e }

# Slow network
td51ddfce = Медленная сеть

# Off: traffic goes as fast as it can
tf0f34db0 = Выключено: трафик идёт с максимальной скоростью

# Custom
t9525b78e = Свои значения

# Only these hosts
tfe16992b = Только эти хосты

# Apply
tca77bbdb = Применить

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Действует со следующего запроса. Задержка добавляется к каждому обмену туда-обратно; скорость — это канал, общий для всех замедленных потоков; потерянный фрагмент ждёт повторной отправки, как в TCP. Остальные хосты не замедляются.

# Header line {line} has no ":"
t891b076d = В строке заголовка { $line } нет ":"

# Header line {line} has no name
te3247b96 = В строке заголовка { $line } нет имени

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
        [one] { $n } байт — не текст: передаётся как пришёл.
        [few] { $n } байта — не текст: передаются как пришли.
        [many] { $n } байт — не текст: передаются как пришли.
       *[other] { $n } байта — не текст: передаются как пришли.
    }

# Nothing is paused
t8cf68b4c = Ничего не на паузе

# "{status}" is not a status
t40a529af = "{ $status }" — не статус

# REQUEST
t1f3f6dd2 = ЗАПРОС

# RESPONSE
tc85f77de = ОТВЕТ

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Ничего не на паузе. Добавьте правило «Пауза» на экране «Правила», чтобы останавливать трафик здесь.

# for {method} {url}
t093a86f6 = для { $method } { $url }

# The request, before the server gets it
tc840fed6 = Запрос, до того как его получит сервер

# The response, before the app gets it
t6683d72a = Ответ, до того как его получит приложение

# Send as it came
tb593e730 = Отправить как есть

# Drop
t108524ac = Отбросить

# Paused
tde281feb = На паузе

# Nothing is waiting.
tea613769 = Ничего не ждёт.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
        [one] { $n } ждёт отправки.
        [few] { $n } ждут отправки.
        [many] { $n } ждут отправки.
       *[other] { $n } ждут отправки.
    }

# Send all as they came
tdce25640 = Отправить все как есть

# The list was not saved: {e}
tfceba380 = Список не сохранён: { $e }

# {count} service: {names} | {count} services: {names}
tc1b8764c =
    { $n ->
        [one] { $count } сервис: { $names }
        [few] { $count } сервиса: { $names }
        [many] { $count } сервисов: { $names }
       *[other] { $count } сервиса: { $names }
    }

# Add
t9dc3aa14 = Добавить

# https://api.example.com or http://localhost:50051
t14f9744c = https://api.example.com или http://localhost:50051

# Reading…
t6aad8485 = Чтение…

# {count} file | {count} files
tde674bd8 =
    { $n ->
        [one] { $count } файл
        [few] { $count } файла
        [many] { $count } файлов
       *[other] { $count } файла
    }

# {count} message | {count} messages
t8a1cd2f6 =
    { $n ->
        [one] { $count } сообщение
        [few] { $count } сообщения
        [many] { $count } сообщений
       *[other] { $count } сообщения
    }

# {count} service | {count} services
t51f71902 =
    { $n ->
        [one] { $count } сервис
        [few] { $count } сервиса
        [many] { $count } сервисов
       *[other] { $count } сервиса
    }

# and {count} more problem | and {count} more problems
t591ce5ea =
    { $n ->
        [one] и ещё { $count } проблема
        [few] и ещё { $count } проблемы
        [many] и ещё { $count } проблем
       *[other] и ещё { $count } проблемы
    }

# Remove
t21a5901d = Удалить

# Protobuf schemas
ta4401d1a = Схемы Protobuf

# Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
te5f4f07c = Папки с файлами .proto или descriptor set, созданные через protoc -o, чтобы показывать тела Protobuf и gRPC с именами полей. Вызов gRPC находит свои сообщения по пути, остальной Protobuf — по messageType в его типе. Они перечитываются при изменении.

# Add a folder…
tb5930e43 = Добавить папку…

# Add files…
ted08dbe5 = Добавить файлы…

# Read again
t2f0dcc5d = Перечитать

# Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.
tf6948a5a = Или запросите файлы у сервера с gRPC reflection. https:// идёт через tracewing; http:// — gRPC без TLS, напрямую к серверу.

# Asking…
tf9cf22fe = Запрос…

# Fetch
t533a3183 = Получить

# Find a message
te6f78626 = Найти сообщение

# Read as {message}
t87b6a380 = Читать как { $message }

# Read as…
t1b90af95 = Читать как…

# Requests to {path} on {host} are read as the message chosen.
t8826d766 = Запросы к { $path } на { $host } читаются как выбранное сообщение.

# Answers from {path} on {host} are read as the message chosen.
tbbe012da = Ответы от { $path } на { $host } читаются как выбранное сообщение.

# Automatic (gRPC path or messageType)
t134c9a06 = Автоматически (путь gRPC или messageType)

# and {count} more: type to narrow them | and {count} more: type to narrow them
t70ff68f9 =
    { $n ->
        [one] и ещё { $count }: введите, чтобы сузить
        [few] и ещё { $count }: введите, чтобы сузить
        [many] и ещё { $count }: введите, чтобы сузить
       *[other] и ещё { $count }: введите, чтобы сузить
    }

# Write the server's address first
t4f68f91a = Сначала введите адрес сервера

# "{address}" is not a server's address
t8c4d03a6 = «{ $address }» — не адрес сервера

# The server has no services to describe
tfa5b7ad8 = У сервера нет сервисов для описания

# The server sent no files for its services
tdca19bdc = Сервер не прислал файлы своих сервисов

# The server answered {status}: it may not offer gRPC reflection
t345fdb40 = Сервер ответил { $status }: возможно, у него нет gRPC reflection

# The server answered {code}: it may not offer gRPC reflection. {message}
tb629329e = Сервер ответил { $code }: возможно, у него нет gRPC reflection. { $message }

# The server answered nothing: it may not offer gRPC reflection
t6bbb6207 = Сервер ничего не ответил: возможно, у него нет gRPC reflection

# Block
tbd074002 = Блокировать

# Answer from a file
td8edab50 = Ответить из файла

# Send to another host
te3cd85d7 = Отправить на другой хост

# Set a request header
tfad4aaaa = Задать заголовок запроса

# Set a response header
tcbbc20d0 = Задать заголовок ответа

# Remove a request header
t5228dd84 = Удалить заголовок запроса

# Remove a response header
teeb5735a = Удалить заголовок ответа

# Set a query parameter
t73d9419b = Задать параметр запроса

# Remove a query parameter
t12633da9 = Удалить параметр запроса

# Change the request body
t41101201 = Изменить тело запроса

# Change the response body
tdd259b73 = Изменить тело ответа

# Change the status
t4bbdfba8 = Изменить статус

# Pause the request
t40ecc39b = Приостановить запрос

# Pause the response
t18a34799 = Приостановить ответ

# Status to answer with, like 403
t8f3ebb6c = Статус ответа, например 403

# Full path of a file or folder
t9fb9ae4c = Полный путь к файлу или папке

# Where to, like http://localhost:3000
tdfedd004 = Куда, например http://localhost:3000

# Header's name
t80793085 = Имя заголовка

# Its value
t43968778 = Его значение

# Parameter's name
t313c8e35 = Имя параметра

# Text to find, or empty for the whole body
t8255fc87 = Текст для поиска, или пусто для всего тела

# What to put in its place
t03671fd5 = Что поставить на его место

# Status the client gets, like 500
t5e4d3088 = Статус, который получит клиент, например 500

# "{first}" is not a status
td43128d3 = "{ $first }" — не статус

# Block with {status}
t1727e984 = Блокировать с { $status }

# Answer from {path}
taefc08fa = Ответить из { $path }

# Send to {to}
t5e256873 = Отправить на { $to }

# Request: {header}: {value}
taa230766 = Запрос: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Ответ: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Запрос: без { $header }

# Response: no {header}
t23960a2e = Ответ: без { $header }

# Query: {name}={value}
t0ee6c1c0 = Query: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Query: без { $name }

# Status {status}
tad048e53 = Статус { $status }

# Request body: {replace}
te4856c36 = Тело запроса: { $replace }

# Response body: {replace}
t8600dc42 = Тело ответа: { $replace }

# Request body: {find} → {replace}
taec471a9 = Тело запроса: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Тело ответа: { $find } → { $replace }

# Name
t0fe07306 = Имя

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, например api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Метод, или пусто для всех

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
        [one] { $n } правило записано в { $path }
        [few] { $n } правила записано в { $path }
        [many] { $n } правил записано в { $path }
       *[other] { $n } правила записано в { $path }
    }

# The rules were not written: {e}
te7ac257f = Правила не записаны: { $e }

# Import
t573e0af4 = Импорт

# {path} could not be read: {e}
t32a3ce94 = Не удалось прочитать { $path }: { $e }

# Nothing was imported: {e}
tb9f7257a = Ничего не импортировано: { $e }

# Edit
tc2c76cb1 = Изменить

# Delete
t5797ea6a = Удалить

# Change this rule
t9160f393 = Изменить это правило

# A new rule
t55a722a0 = Новое правило

# It waits under Paused until you send it on, changed or not.
tca566a57 = Он ждёт в разделе «На паузе», пока вы не отправите его дальше, с изменениями или без.

# Save
t4d2d5d68 = Сохранить

# Cancel
t35afca3b = Отмена

# Rules
t428b2d80 = Правила

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Правила действуют со следующего запроса, и пока запись на паузе. Сверху вниз: побеждает первая блокировка, файл или хост, а все остальные подходящие правила применяются по порядку. Правило, меняющее тело или статус, меняет то, что прислал сервер.

# Import rules…
t5edc205d = Импортировать правила…

# Export rules…
t6e09ca20 = Экспортировать правила…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
        [one] Добавлено { $n } правило
        [few] Добавлено { $n } правила
        [many] Добавлено { $n } правил
       *[other] Добавлено { $n } правила
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
        [one] { $n } уже есть
        [few] { $n } уже есть
        [many] { $n } уже есть
       *[other] { $n } уже есть
    }

# left out: {why}
ta81e95e2 = пропущено: { $why }

# Copy all
t6aef5c37 = Копировать всё

# Too large to copy all
t2fe8ff9f = Слишком много, чтобы копировать всё

# No flows
t6dc463eb = Нет потоков

# Name it to keep it
t52c1a402 = Дайте имя, чтобы сохранить

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · записано другой версией, поэтому не открывается

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
        [one] { $count } поток · { $size }
        [few] { $count } потока · { $size }
        [many] { $count } потоков · { $size }
       *[other] { $count } потока · { $size }
    }

# Recording now
t3cb3e6fc = Идёт запись

# Export HAR
t4d6458cc = Экспорт HAR

# Delete for good?
ta7737f13 = Удалить навсегда?

# Sessions
tb69a8aec = Сессии

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Сессия с именем хранится, пока вы её не удалите. Из остальных остаются { $kept } самых новых.

# Working…
td5507854 = Выполняется…

# Open a session file
tae8622fc = Открыть файл сессии

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, Fiddler .saz или Charles .chlsj

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = В экспортированных HAR нет cookie, токенов, ключей и паролей — ни в заголовках, ни в URL, ни в телах

# Other devices
t9e7d8362 = Другие устройства

# Let other devices use the proxy
t13e626b3 = Разрешить другим устройствам использовать прокси

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = Тогда телефоны и другие компьютеры в вашей сети смогут отправлять сюда свой трафик. Как и любой другой в этой сети: не включайте это в сети, которой не доверяете.

# Servers
tcd0cf113 = Серверы

# Trust what this computer trusts
tfea2d91b = Доверять тому, чему доверяет этот компьютер

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = К серверам с сертификатом от центра сертификации, установленного на этом компьютере (например, корпоративного), подключение идёт так же, как у браузеров этого компьютера. Если выключено, доверие есть только публичным центрам сертификации. Вступает в силу после перезапуска tracewing.

# AI tools
t294d7ea4 = ИИ-инструменты

# Let AI tools see tokens and cookies
te1d26d3c = Разрешить ИИ-инструментам видеть токены и cookie

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = ИИ-инструменты читают записанный трафик через MCP-сервер tracewing. Если выключено, значения Authorization, Cookie и Set-Cookie в получаемые ими данные не попадают. Применяется сразу.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Не пускать приложения на HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = HTTP/3 работает поверх UDP, который не передаёт ни один прокси, поэтому приложение, использующее его, идёт в обход tracewing. Эта настройка убирает из ответов серверов предложения HTTP/3, и приложения остаются на HTTP/2. В потоках предложения по-прежнему видны такими, какими их отправили серверы.

# Size of a session
ta20fa102 = Размер сессии

# Past this, the oldest flows are dropped.
td2914f46 = При превышении самые старые потоки удаляются.

# Room to leave on the disk
t8674a3b3 = Свободное место на диске

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Если свободного места меньше, потоки по-прежнему отображаются, но тела не сохраняются.

# Earlier sessions to keep
td356194a = Сколько хранить прошлых сессий

# Older ones are deleted when a new session starts.
t66556513 = Более старые удаляются при начале новой сессии.

# Bodies
tdb4d0409 = Тела

# Most kept of one body
t777508dd = Максимум хранимого из одного тела

# It still reaches the client whole.
t386fee28 = Клиент всё равно получает его целиком.

# Most unpacked for viewing
tc3fe2efd = Максимум распаковки для просмотра

# Stops a small download that unpacks to many GB.
tf9c49515 = Не даёт маленькой загрузке распаковаться во много GB.

# Small body size
ta8996af1 = Размер маленького тела

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Тела меньшего размера ждут в памяти и делят один файл. Более крупные получают собственный файл по мере поступления.

# Compress plain text on disk
tee503c2e = Сжимать простой текст на диске

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Занимает примерно вдвое меньше места; пропускается, если трафик идёт слишком быстро.

# Compression level
td386c9ef = Уровень сжатия

# 1 to 19. Higher is smaller and slower.
td42a987d = От 1 до 19. Выше — меньше и медленнее.

# Search
tc646a2c9 = Поиск

# Search URLs
t0c3e0b3d = Поиск по URL

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Находит любую часть URL по мере ввода. Обходится дёшево.

# Search bodies
t8eec0293 = Поиск по телам

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Находит слова в телах. Может занять на диске столько же места, сколько сами тела.

# Searched part of each body
tab293b21 = Просматриваемая часть каждого тела

# Words past this are not found. More takes more disk.
t0bf65f4a = Слова дальше этого предела не находятся. Больше — больше места на диске.

# Under heavy traffic
ta99888cb = При высокой нагрузке

# Memory for bodies waiting to be written
t7d6ff799 = Память для тел, ожидающих записи

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = При превышении потоки отображаются без тел. Трафик никогда не задерживается.

# Memory for large bodies waiting to be written
t07b9de08 = Память для больших тел, ожидающих записи

# Past this, large bodies stop being kept.
tfe61a4f3 = При превышении большие тела перестают сохраняться.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }" — не целое число

# {key}: must not be negative
t9c4debaa = { $key }: не может быть отрицательным

# After a restart
t58dcea2d = После перезапуска

# Every setting is back to its default.
t763785e7 = Все настройки возвращены к значениям по умолчанию.

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d =
    { $n ->
        [one] { $n } изменение вступит в силу после перезапуска tracewing.
        [few] { $n } изменения вступят в силу после перезапуска tracewing.
        [many] { $n } изменений вступят в силу после перезапуска tracewing.
       *[other] { $n } изменения вступят в силу после перезапуска tracewing.
    }

# Settings
t4b058728 = Настройки

# Undo
t71fd4acf = Отменить

# Back to defaults
ta3162a13 = Сбросить настройки

# Hosts not to decrypt
tb457f0b3 = Хосты без расшифровки

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = По одному на строку: хост или *.example.com для него и всего, что под ним. Их трафик проходит без чтения. Список применяется сразу.

# Language
t9a73db9b = Язык

# System ({system_name})
tb5c7a1d4 = Системный ({ $system_name })

# Applies at once.
tefb7a0f9 = Применяется сразу.

# Your browser
tb5a48dd2 = Ваш браузер

# Its private key is in the system keychain.
t62a47808 = Его закрытый ключ хранится в системной связке ключей.

# Its private key is in ca.key, which only you can read.
td9c6165d = Его закрытый ключ хранится в ca.key, который можете прочитать только вы.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = Его закрытый ключ хранится в ca.key, который можете прочитать только вы, так как системная связка ключей не может его принять: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Откройте страницу. Если браузер предупреждает о сертификате или не находит страницу, один из шагов выше не выполнен для этого браузера.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } работает через tracewing и доверяет ему.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } работает через tracewing, но открыл страницу без HTTPS. Откройте её снова через https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = Телефон или другой компьютер тоже может отправлять сюда свой трафик. Включите «Разрешить другим устройствам использовать прокси» в Настройках.

# this computer's address, port {port}
t2188b3f4 = адрес этого компьютера, порт { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = На устройстве укажите прокси для Wi-Fi: { $addr }. Затем откройте там http://{ $host }, скачайте сертификат и сделайте его доверенным. Потоки устройства показывают его адрес и фильтруются через device:. Дайте устройству имя по одному из них, в разделе Подключение.

# Looking for it…
t50ee7452 = Поиск…

# Installing. The system may ask you to confirm.
ta41c38ec = Установка. Система может запросить подтверждение.

# Removing. The system may ask you to confirm.
t62b22726 = Удаление. Система может попросить подтверждение.

# This computer trusts it.
t27d6bacc = Этот компьютер ему доверяет.

# Installed in: {places}.
tb5bcc244 = Установлен в: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Инструменты командной строки ему доверяют, но у браузеров собственные хранилища. Пока его там нет, HTTPS через tracewing в браузере не работает.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing читает HTTPS с помощью сертификата, созданного только для этого компьютера. Пока ему не доверяют, HTTPS через tracewing не работает.

# Trust the certificate
t26618699 = Доверять сертификату

# Install
tb50cbd40 = Установить

# Also for command-line tools
t0db8faea = Также для инструментов командной строки

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Удалить сертификат из всех хранилищ этого компьютера? HTTPS через tracewing перестанет работать, пока его не установят снова. tracewing сохранит его, чтобы установить позже.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = В Linux браузерам нужен certutil, чтобы им можно было сказать доверять сертификату. Установите его, затем нажмите здесь «Установить»:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = В Linux браузерам нужен certutil, чтобы им можно было сказать доверять сертификату. Установите NSS-инструменты своего дистрибутива, затем нажмите здесь «Установить».

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = В этом окружении рабочего стола нет системного прокси, который tracewing может изменить. Откройте ниже браузер отдельно в разделе «Браузеры» или терминал.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = В { $desktop } нет системного прокси, который tracewing может изменить. Откройте ниже браузер отдельно в разделе «Браузеры» или терминал.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Приложения, использующие системную настройку, теперь идут через { $addr }. Она вернётся к прежнему значению, когда tracewing закроется или аварийно завершится.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Направляет трафик этого компьютера через tracewing для приложений, использующих системную настройку. Или сами направьте приложение на { $addr }.

# Turn on the system proxy
t52b88441 = Включить системный прокси

# Turn off
tcd03e04b = Выключить

# Turn on
t65a0c5a7 = Включить

# Check it in your browser
tbed6e83a = Проверьте в браузере

# Open the test page
te4e36f65 = Открыть тестовую страницу

# Set up
t43c06abc = Настройка

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Три шага, чтобы увидеть HTTPS-трафик этого компьютера.

# See traffic
t980eee79 = Смотреть трафик

# the browsers' own stores
tddf6836c = собственные хранилища браузеров

# Trusts the certificate.
tc5bba858 = Доверяет сертификату.

# Does not trust the certificate yet.
tf2a0b8c5 = Пока не доверяет сертификату.

# Looking…
t284579ba = Поиск…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Профиля пока нет. Откройте браузер один раз, затем поищите снова.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 =
    { $n ->
        [one] { $holding } из { $n } профиля доверяют сертификату.
        [few] { $holding } из { $n } профилей доверяют сертификату.
        [many] { $holding } из { $n } профилей доверяют сертификату.
       *[other] { $holding } из { $n } профиля доверяют сертификату.
    }

# Has not opened the test page yet.
t67f9cb49 = Тестовую страницу ещё не открывал.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Открыл тестовую страницу через tracewing, по HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Открыл тестовую страницу через tracewing, но без HTTPS.

# Browsers
td580efd4 = Браузеры

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Каждый браузер сам решает, чему доверять и куда отправлять трафик. «Проверить» открывает в нём тестовую страницу. «Открыть отдельно» запускает его с собственным профилем, который идёт через tracewing, не трогая системный прокси.

# Look again
t98416222 = Поискать снова

# No browser found on this computer.
t0e46bb34 = На этом компьютере браузеры не найдены.

# Check
t592c8de7 = Проверить

# Open on its own
t0da7783c = Открыть отдельно

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = UDP-порт 443 на этом компьютере заблокирован, поэтому приложения откатываются с HTTP/3 на HTTP/2 и идут через tracewing. Блокировка снимается, когда tracewing закрывается или аварийно завершается.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Некоторые приложения используют HTTP/3 поверх UDP и обходят прокси. Блокировка UDP-порта 443 на этом компьютере заставляет их откатиться на HTTP/2 через tracewing. Нужен пароль администратора; блокировка снимается, когда tracewing закрывается. VPN, использующий UDP-порт 443, на это время перестаёт работать.

# Apps on HTTP/3
t2cfe5270 = Приложения на HTTP/3

# Stop blocking
t318358d8 = Снять блокировку

# Block HTTP/3
t88231e40 = Блокировать HTTP/3

# Or one app at a time
t91c63f1c = Или по одному приложению

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Терминал, который идёт через tracewing, а системный прокси и остальные приложения остаются как есть. Для браузера то же самое делает «Открыть отдельно» в разделе «Браузеры», с собственным профилем.

# Open a terminal
t4b629c72 = Открыть терминал

# iPhone and iPad
ta550cc97 = iPhone и iPad

# Flutter apps
ta04ee397 = Приложения Flutter

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. В Настройках, Wi-Fi, выберите сеть, затем Настройка прокси, Вручную: сервер { $server }, порт { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Отсканируйте код камерой или откройте { $page } в Safari и скачайте сертификат. Разрешите загрузку.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. В Настройках, Основные, VPN и управление устройством, выберите профиль tracewing CA и установите его.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. В Настройках, Основные, Об этом устройстве, Доверие сертификатам, включите полное доверие для tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Откройте https://{ $host } в Safari. Если там сказано, что всё работает, приложения, использующие системные настройки, идут через tracewing.

# When you are done
t62466f08 = Когда закончите

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. В Настройках, Wi-Fi выберите сеть, затем Настройка прокси: Выкл. Если оставить, устройство останется без интернета в этой сети, когда tracewing закроется.

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. В Настройках, Основные, VPN и управление устройством выберите tracewing CA и удалите профиль. Полное доверие уйдёт вместе с ним.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. В Настройках, Wi-Fi, измените сеть и в разделе Расширенные настройки задайте прокси Вручную: хост { $host }, порт { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Отсканируйте код или откройте { $page } в браузере и скачайте сертификат.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. В Настройках, Безопасность, Шифрование и учетные данные (названия зависят от производителя), выберите Установка сертификата, затем Сертификат ЦС, и укажите tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Начиная с Android 7, приложения доверяют сертификатам, добавленным пользователем, только если сами это разрешают. Для отладочной сборки своего приложения добавьте res/xml/network_security_config.xml:

# and point to it from AndroidManifest.xml:
t02ad844b = и сошлитесь на него в AndroidManifest.xml:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Другие приложения по-прежнему отклоняют сертификат: их трафик виден как неудачные потоки.

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. В Настройках, Wi-Fi измените сеть и верните прокси в положение Нет. Если оставить, устройство останется без интернета в этой сети, когда tracewing закроется.

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. В Настройках, Безопасность, Шифрование и учётные данные, Пользовательские учётные данные выберите tracewing и удалите. Удалите и tracewing.crt из Загрузок.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = HttpClient из Dart, который используют http и Dio во Flutter, не следует настройке прокси телефона, а на Android не доверяет сертификатам, добавленным пользователем. В отладочной сборке направьте его сюда:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback пропускает любой сертификат, поэтому он только для отладочных сборок.

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = Уберите TracewingProxy и его строку в main() перед выпуском, хотя вне отладочных сборок они ничего не делают. На телефоне ничего не менялось.

# Open settings
t1336b4d6 = Открыть настройки

# Phones and other devices
t2e3c591c = Телефоны и другие устройства

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Телефон, планшет или другой компьютер в этой сети может работать через tracewing по адресу { $proxy }. Код открывает страницу с сертификатом. Потоки устройства показывают его адрес, фильтруются через device:, а имя устройству можно дать по одному из них, в разделе Подключение.

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = Другие устройства не могут подключиться к tracewing по одному из адресов этого компьютера: { $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f =
    { $n ->
        [one] { $n } устройство прошло через tracewing ({ $names }). Когда закончите, выключите на нём прокси Wi-Fi, как описано ниже: если оставить, оно останется без интернета в этой сети, когда tracewing закроется.
        [few] { $n } устройства прошли через tracewing ({ $names }). Когда закончите, выключите на них прокси Wi-Fi, как описано ниже: если оставить, они останутся без интернета в этой сети, когда tracewing закроется.
        [many] { $n } устройств прошли через tracewing ({ $names }). Когда закончите, выключите на них прокси Wi-Fi, как описано ниже: если оставить, они останутся без интернета в этой сети, когда tracewing закроется.
       *[other] { $n } устройства прошли через tracewing ({ $names }). Когда закончите, выключите на них прокси Wi-Fi, как описано ниже: если оставить, они останутся без интернета в этой сети, когда tracewing закроется.
    }

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Готово. Сертификат находится среди системных до перезагрузки устройства.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Готово. Сертификат в Загрузках: в только что открытых Настройках выберите Шифрование и учетные данные, Установка сертификата, Сертификат ЦС и укажите tracewing.crt.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Приложения, доверяющие только системным сертификатам, всё равно будут его отклонять.

# An emulator.
t14b6b86d = Эмулятор.

# On USB.
t67dafc14 = По USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Подтвердите запрос на отладку по USB на устройстве, затем поищите снова.

# Still starting. Look again in a moment.
t13f68b64 = Ещё запускается. Поищите снова чуть позже.

# adb says it is {state}.
t2b7d71da = adb сообщает состояние: { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Настраивает прокси на эмуляторе Android или телефоне, подключённом по USB с включённой отладкой по USB, и устанавливает сертификат. Трафик идёт через ADB, поэтому им не нужно быть в этой сети.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = adb не найден. Установите Android Studio или Android platform tools, затем поищите снова.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Нет подключённых устройств. Запустите эмулятор или подключите телефон с включённой отладкой по USB, затем поищите снова.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Настройте устройство, чтобы оно отправляло сюда свой трафик. Отмена или закрытие tracewing снова убирает с него прокси.

# Android emulators and USB devices
t379e433c = Эмуляторы Android и USB-устройства

# Find devices
t54d4af39 = Найти устройства

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = Системный прокси и блокировка HTTP/3 возвращаются как было, когда tracewing закрывается или падает, как и Android-устройство, настроенное по USB. Сертификат остаётся доверенным, пока «Удалить» на шаге 1 его не уберёт. Телефон, настроенный вручную, хранит прокси Wi-Fi и сертификат, пока их не уберут на нём самом: как это сделать, сказано в инструкции для каждого устройства. Другие устройства могут подключаться к прокси этого компьютера, пока это не выключено в Настройках.

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code и другие ИИ-инструменты могут получать список потоков, записанных tracewing, и читать их через его MCP-сервер. Токены и cookie не передаются, если Настройки этого не разрешают. В Claude Code:

# Elsewhere, in the tool's MCP settings:
tec38833c = В других инструментах, в их настройках MCP:

# Flows
t4db77db2 = Потоки

# Choose the flow above or below
tecb3adec = Выбрать поток выше или ниже

# The first or the last flow
t7e278a00 = Первый или последний поток

# A page up or down
t23b1814b = На страницу вверх или вниз

# Filter
tf4a9c097 = Фильтр

# Pause or resume recording
t6e21519a = Приостановить или продолжить запись

# Clear the list
tf2a55d1f = Очистить список

# Compare the flow clicked with the chosen one
te42cd067 = Сравнить нажатый поток с выбранным

# The chosen flow
te0a729de = Выбранный поток

# Its request
tb3970784 = Его запрос

# Its response
t221fc168 = Его ответ

# Send it again
tef7d1944 = Отправить повторно

# Edit it and send it
t6bc99e5c = Изменить и отправить

# Copy it as cURL
tc99a11a3 = Копировать как cURL

# The part before or after the one shown
tdf4221d9 = Раздел до или после показанного

# Save the body shown
t20d9b7c0 = Сохранить показанное тело

# Select all of the part shown
tf9168168 = Выделить всё в показанной части

# Copy the selected text
t00b090d3 = Копировать выделенный текст

# Send it on, as edited
t8c35fa5a = Отправить с правками

# Send it on as it was
t16f7abb5 = Отправить как было

# Drop it
t4e7f48b9 = Отбросить

# The window
t2af18488 = Окно

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Настройка, Трафик, Правила, На паузе, Конструктор, Сессии, Настройки

# Show or hide the sidebar
t67adca04 = Показать или скрыть боковую панель

# Show or hide the inspector
ta5a7b9e9 = Показать или скрыть инспектор

# The inspector beside the list or below it
t51e214f7 = Инспектор сбоку от списка или под ним

# The next or the previous field
ta4b4bd5e = Следующее или предыдущее поле

# These shortcuts
tbdf3b62b = Эти сочетания клавиш

# Close them
tf3a4b1ef = Закрыть их

# Keyboard shortcuts
t31eaddb7 = Сочетания клавиш

# Close
tcd86acc3 = Закрыть

# The certificate: {e}
t0235648c = Сертификат: { $e }

# The terminal
tf542b2f4 = Терминал

# The system proxy: {e}
tc99726ce = Системный прокси: { $e }

# Blocking HTTP/3: {e}
t024ec871 = Блокировка HTTP/3: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx или текст в телах

# A name, as iPhone test
t19f89214 = Имя, например iPhone тест

# The search index: {e}
te3c971c9 = Поисковый индекс: { $e }

# Click the flow to compare #{id} with.
tac1ec1d2 = Нажмите на поток, чтобы сравнить его с #{ $id }.

# An earlier session: {name}
t73446a1d = Прошлая сессия: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Только чтение. Поиск охватывает больше по мере построения индекса. Запись продолжается.

# Read-only. Recording goes on.
te2f35d9e = Только чтение. Запись продолжается.

# Back to live
t317c0453 = Вернуться к живой записи

# {shown} of {total}
tf2cc0563 = { $shown } из { $total }

# Recording
te75cf20c = Запись

# System proxy off · {addr}
t7e813d25 = Системный прокси выключен · { $addr }

# System proxy…
t29e7dcd0 = Системный прокси…

# System proxy on · {addr}
t319ad6b9 = Системный прокси включён · { $addr }

# Apps
t53d91aad = Приложения

# Devices
t27acf320 = Устройства

# All hosts
tfc7bfe37 = Все хосты

# No host has this in its name
t838c1d44 = Нет хостов с этим в имени

# Hidden
t3c2dfe59 = Скрытые

# Hosts
tcf9aefb4 = Хосты

# Newest
t324b6d0b = Новые

# No flow fits this filter
t7eb295f4 = Нет потоков под этот фильтр

# Nothing yet
tbd00747c = Пока ничего

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Включите системный прокси выше или направьте приложение на { $addr }. Для проверки: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Сбоку от списка

# Below the list
t23658daf = Под списком

# Drop it on a place · Esc puts it back
t0fa632ba = Перетащите в нужное место · Esc — вернуть

# Traffic
t4b589266 = Трафик

# Certificate trusted on this computer
t7b93f551 = Этот компьютер доверяет сертификату

# Checking the certificate…
tbf8942c3 = Проверка сертификата…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Установка сертификата; система может запросить подтверждение…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Удаление сертификата; система может попросить подтверждение…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS не будет работать, пока сертификату не доверяют: настройка

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml не применён: { $problem }

# A setting was not saved: {problem}
tff1a5018 = Настройка не сохранена: { $problem }

# The proxy stopped: {failure}
t04dcff11 = Прокси остановлен: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = Диск почти заполнен: тела не сохраняются

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
        [one] { $count } поток не записан
        [few] { $count } потока не записано
        [many] { $count } потоков не записано
       *[other] { $count } потока не записано
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
        [one] { $count } поток без тела
        [few] { $count } потока без тел
        [many] { $count } потоков без тел
       *[other] { $count } потока без тел
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
        [one] { $count } самый старый поток удалён, чтобы не превысить { $size }
        [few] { $count } самых старых потока удалено, чтобы не превысить { $size }
        [many] { $count } самых старых потоков удалено, чтобы не превысить { $size }
       *[other] { $count } самых старых потока удалено, чтобы не превысить { $size }
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
        [one] { $count } поток
        [few] { $count } потока
        [many] { $count } потоков
       *[other] { $count } потока
    }

# Export as HAR
t95b6b766 = Экспортировать как HAR

# Shortcuts {keys}
t4ab0ecb4 = Сочетания клавиш { $keys }

# Columns
tea156fdc = Столбцы
