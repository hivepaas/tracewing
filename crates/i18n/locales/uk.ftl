# tracewing in Ukrainian. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {size} more
ta8dc6eab = … ще { $size }

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

# Appearance: {now}. Click for {next}.
t332cfa6d = Вигляд: { $now }. Натисніть, щоб змінити на: { $next }.

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

# Size
ta6478e7c = Розмір

# Time
tdfe4e404 = Час

# Number
t61e9eac0 = Номер

# "{method}" is not a method
tacdcf5cb = "{ $method }" не є методом

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = тіло запиту не збережено повністю, тому його не можна надіслати знову

# name: value, one a line
t4df37bea = назва: значення, по одному в рядку

# No body
tfa4c2902 = Без тіла

# No flow is chosen
t550b0a98 = Потік не вибрано

# This flow: {e}
t296762f8 = Цей потік: { $e }

# Request
t82454152 = Запит

# Headers
t334f5989 = Заголовки

# Body
t428a1095 = Тіло

# Sending…
te946e7bf = Надсилання…

# Send
tf28e14cf = Надіслати

# New
t8b242db1 = Новий

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … ще байтів: { $count }; усі вони є в потоці на вкладці «Трафік».

# Response
t55a982fe = Відповідь

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

# Not decrypted
t8090a1ba = Не розшифровано

# open
td35ec4c9 = відкрито

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

# Expires
tc7aa0523 = Спливає

# Issued
tb1835ecc = Видано

# Not before
td10dacd9 = Не раніше

# in {amount}
tfda57c3e = через { $amount }

# {amount} ago
t5f968014 = { $amount } тому

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

# URL
t7569633e = URL

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

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Нічого не призупинено. Додайте правило «Призупинити» на екрані «Правила», щоб зупиняти трафік тут.

# for {method} {url}
t093a86f6 = для { $method } { $url }

# The request, before the server gets it
tc840fed6 = Запит до того, як його отримає сервер

# The response, before the app gets it
t6683d72a = Відповідь до того, як її отримає застосунок

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

# Pause the request
t40ecc39b = Призупинити запит

# Pause the response
t18a34799 = Призупинити відповідь

# Status to answer with, like 403
t8f3ebb6c = Статус для відповіді, наприклад 403

# Full path of a file or folder
t9fb9ae4c = Повний шлях до файлу або папки

# Where to, like http://localhost:3000
tdfedd004 = Куди, наприклад http://localhost:3000

# Header's name
t80793085 = Назва заголовка

# Its value
t43968778 = Його значення

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

# Add
t9dc3aa14 = Додати

# Cancel
t35afca3b = Скасувати

# Rules
t428b2d80 = Правила

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every header rule that fits applies.
tb44c7227 = Діють з наступного запиту, навіть коли запис призупинено. Згори донизу: перемагає перше правило блокування, файлу чи хоста, а всі відповідні правила заголовків застосовуються.

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

# Exported HAR files leave out tokens and cookies
td4a2f543 = Експортовані файли HAR не містять токенів і cookie

# Other devices
t9e7d8362 = Інші пристрої

# Let other devices use the proxy
t13e626b3 = Дозволити іншим пристроям використовувати проксі

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust. Takes effect when tracewing starts again.
t2e14229d = Тоді телефони й інші комп’ютери у вашій мережі зможуть надсилати сюди свій трафік. Як і будь-хто інший у цій мережі, тому не вмикайте це в мережі, якій ви не довіряєте. Набуває чинності після перезапуску tracewing.

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

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Не пускати застосунки на HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them. Takes effect when tracewing starts again.
t2aa7be0d = HTTP/3 працює поверх UDP, який жоден проксі не передає, тож застосунок, що використовує його, обходить tracewing. Це налаштування прибирає з відповідей серверів пропозиції HTTP/3, і застосунки залишаються на HTTP/2. Потоки все одно показують пропозиції так, як їх надіслали сервери. Набуває чинності після перезапуску tracewing.

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

# Saved. The limits apply the next time tracewing starts.
tc8bd2065 = Збережено. Обмеження почнуть діяти після наступного запуску tracewing.

# Settings
t4b058728 = Налаштування

# Back to defaults
ta3162a13 = Відновити типові

# Hosts not to decrypt
tb457f0b3 = Хости, які не розшифровувати

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies as soon as it is saved.
t7647c310 = По одному в рядку: хост або *.example.com для нього й усього під ним. Їхній трафік проходить непрочитаним. Список діє одразу після збереження.

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

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings, then start tracewing again.
t9f95b7b8 = Телефон або інший комп’ютер також може надсилати сюди свій трафік. Увімкніть «Дозволити іншим пристроям використовувати проксі» в Налаштуваннях, а потім перезапустіть tracewing.

# this computer's address, port {port}
t2188b3f4 = адреса цього комп’ютера, порт { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = На пристрої вкажіть проксі Wi-Fi { $addr }. Потім відкрийте там http://{ $host }, отримайте сертифікат і довіртеся йому. Потоки пристрою показують його адресу, і їх можна фільтрувати за допомогою device:. Дайте пристрою назву з будь-якого з них на вкладці «З’єднання».

# Looking for it…
t50ee7452 = Пошук…

# Installing. The system may ask you to confirm.
ta41c38ec = Встановлення. Система може попросити підтвердження.

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

# A terminal, or a browser with a profile of its own, that goes through tracewing while the system proxy and every other app are left as they are.
t259dbb0e = Термінал або браузер з окремим профілем, що працює через tracewing, тоді як системний проксі й усі інші застосунки залишаються без змін.

# Open a terminal
t4b629c72 = Відкрити термінал

# Open {name}
tbd869c4a = Відкрити { $name }

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

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = HttpClient у Dart, який використовують http і Dio у Flutter, не враховує налаштування проксі телефона, а на Android не довіряє сертифікатам, доданим користувачем. У налагоджувальній збірці спрямуйте його сюди:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback пропускає будь-який сертифікат, тому він лише для налагоджувальних збірок.

# Open settings
t1336b4d6 = Відкрити налаштування

# Phones and other devices
t2e3c591c = Телефони та інші пристрої

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Телефон, планшет або інший комп’ютер у цій мережі може працювати через tracewing за адресою { $proxy }. Код відкриває сторінку з його сертифікатом. Потоки пристрою показують його адресу, їх можна фільтрувати за допомогою device:, а сам пристрій можна назвати з будь-якого з них на вкладці «З’єднання».

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

# Undo
t71fd4acf = Відмінити

# Android emulators and USB devices
t379e433c = Емулятори Android і USB-пристрої

# Look again
t98416222 = Пошукати знову

# Find devices
t54d4af39 = Знайти пристрої

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

# Select all of the part shown
tf9168168 = Виділити все в показаній частині

# Copy the selected text
t00b090d3 = Копіювати виділений текст

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

# These shortcuts
tbdf3b62b = Ці комбінації клавіш

# Close them
tf3a4b1ef = Закрити їх

# Keyboard shortcuts
t31eaddb7 = Комбінації клавіш

# Close
tcd86acc3 = Закрити

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

# The search index: {e}
te3c971c9 = Пошуковий індекс: { $e }

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

# Hosts
tcf9aefb4 = Хости

# All hosts
tfc7bfe37 = Усі хости

# Hidden
t3c2dfe59 = Приховані

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

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS не працюватиме, доки сертифікат не стане довіреним: налаштувати

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml не використано: { $problem }

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
