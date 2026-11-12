# tracewing in German. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 =
    { $n ->
        [one] … { $count } Zeile { $closer }
       *[other] … { $count } Zeilen { $closer }
    }

# … {size} more
ta8dc6eab = … { $size } mehr

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc =
    { $n ->
        [one] { $size } { $format } · { $count } Zeile JSON
       *[other] { $size } { $format } · { $count } Zeilen JSON
    }

# The first {count} rows are shown.
t16aa1c6d = Nur die ersten { $count } Zeilen werden gezeigt.

# Unfold all
t3849d44e = Alles aufklappen

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } Zeile
       *[other] { $size } · { $count } Zeilen
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · wird gelesen: { $percent }%

# Appearance: {now}. Click for {next}.
t332cfa6d = Darstellung: { $now }. Klicken für { $next }.

# Method
t98a2c750 = Methode

# Status
t005ef20f = Status

# App
t81f51acc = App

# Host
t302af18f = Host

# Path
teb66e456 = Pfad

# Operation
t218e02a6 = Operation

# Size
ta6478e7c = Größe

# Time
tdfe4e404 = Zeit

# Number
t61e9eac0 = Nummer

# A body was not kept, so the two cannot be compared.
t9d6abb2a = Ein Body wurde nicht behalten, daher lassen sich die beiden nicht vergleichen.

# {count} field differs | {count} fields differ
t6a8fff7f =
    { $n ->
        [one] { $count } Feld unterscheidet sich
       *[other] { $count } Felder unterscheiden sich
    }

# … {count} line the same | … {count} lines the same
tcc37e7ea =
    { $n ->
        [one] … { $count } gleiche Zeile
       *[other] … { $count } gleiche Zeilen
    }

# Nothing to compare.
t9f32d942 = Nichts zu vergleichen.

# URL
t7569633e = URL

# Protocol
t4a57ebd5 = Protokoll

# Type
td155d06d = Typ

# Sizes
td1a4a19d = Größen

# Body
t428a1095 = Body

# Headers
t334f5989 = Header

# Query
t9e89e783 = Query

# Request
t82454152 = Anfrage

# Response
t55a982fe = Antwort

# Neither has headers kept.
tb43ef094 = Keiner hat behaltene Header.

# Trailers
t7c44fa89 = Trailer

# Neither URL has a query.
t4d8203e0 = Keine der URLs hat eine Query.

# Comparing…
ta8565d31 = Wird verglichen…

# No body
tfa4c2902 = Kein Body

# {kind}, its first {size} compared
t1d894c39 = { $kind }, die ersten { $size } verglichen

# Neither has a body.
tf2c47725 = Keiner hat einen Body.

# The bodies are the same.
t85152e01 = Die Bodys sind gleich.

# {count} field differs. | {count} fields differ.
t0f651f77 =
    { $n ->
        [one] { $count } Feld unterscheidet sich.
       *[other] { $count } Felder unterscheiden sich.
    }

# The first {count} are listed.
taae62a3e = Nur die ersten { $count } sind aufgeführt.

# {count} line differs. | {count} lines differ.
tf57a3157 =
    { $n ->
        [one] { $count } Zeile unterscheidet sich.
       *[other] { $count } Zeilen unterscheiden sich.
    }

# Fields
t3308bd9c = Felder

# Lines
t8a1a00dc = Zeilen

# Compare
td57a0a54 = Vergleichen

# #{a} beside #{b}: what only #{a} has is red, what only #{b} has is green.
t71ce2de9 = #{ $a } neben #{ $b }: Was nur #{ $a } hat, ist rot, was nur #{ $b } hat, ist grün.

# Swap
te519d36e = Tauschen

# Back to Traffic
t644042d2 = Zurück zu Traffic

# "{method}" is not a method
tacdcf5cb = „{ $method }“ ist keine Methode

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = der Body der Anfrage wurde nicht vollständig gespeichert, daher kann sie nicht erneut gesendet werden

# name: value, one a line
t4df37bea = Name: Wert, einer pro Zeile

# No flow is chosen
t550b0a98 = Kein Flow ausgewählt

# This flow: {e}
t296762f8 = Dieser Flow: { $e }

# The body is not a {message}: {e}
t3220a10a = Der Body ist kein { $message }: { $e }

# Protobuf {message}, written as JSON: it is sent as Protobuf.
t83b6cd6e = Protobuf { $message }, als JSON geschrieben: gesendet wird Protobuf.

# Sending…
te946e7bf = Wird gesendet…

# Send
tf28e14cf = Senden

# New
t8b242db1 = Neu

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … { $count } weitere Bytes: Der Flow unter Traffic enthält alle.

# Body · {message}
taa3cc3ff = Body · { $message }

# Compose
t18049f93 = Erstellen

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Eine eigene Anfrage oder eine erneut gesendete. Sie läuft über tracewing, also gelten die Regeln, und sie erscheint auch unter Traffic.

# {version} both sides
t961a5ea0 = { $version } auf beiden Seiten

# {client} to the proxy, {server} to the server
t3d237646 = { $client } zum Proxy, { $server } zum Server

# Queued
t9e437c74 = In Warteschlange

# Connect
tdb89a899 = Verbinden

# Waiting
tc9450e48 = Warten

# Download
t6964bb19 = Download

# No file picker: {e}
t6935fbd3 = Keine Dateiauswahl: { $e }

# Writing the HAR…
t725a0e89 = HAR wird geschrieben…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
        [one] { $n } Flow in { $path } geschrieben
       *[other] { $n } Flows in { $path } geschrieben
    }

# The HAR was not written: {e}
tef8ec569 = Die HAR-Datei wurde nicht geschrieben: { $e }

# Open
t538b10e9 = Öffnen

# Reading {path}…
tc0e7c8f4 = { $path } wird gelesen…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
        [one] , { $n } Eintrag ausgelassen
       *[other] , { $n } Einträge ausgelassen
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
        [one] { $n } Flow aus einer { $format } gelesen
       *[other] { $n } Flows aus einer { $format } gelesen
    }

# The file was not read: {e}
t7f49510f = Die Datei wurde nicht gelesen: { $e }

# it is no longer kept
ta7b4c380 = er wird nicht mehr gespeichert

# {size} saved to {path}
tc6932467 = { $size } in { $path } gespeichert

# The body was not saved: {e}
t189900b4 = Der Body wurde nicht gespeichert: { $e }

# Fiddler session
t9f5c89a3 = Fiddler-Sitzung

# Charles session
tb4f56f2b = Charles-Sitzung

# All
t75afe364 = Alle

# Errors
te9ed60a6 = Fehler

# Media
td10fcaa3 = Medien

# Rule: {rule}
teca2074f = Regel: { $rule }

# Events
t003420a4 = Ereignisse

# JSON lines
te8ce2db0 = JSON-Zeilen

# Not decrypted
t8090a1ba = Nicht entschlüsselt

# open
td35ec4c9 = offen

# Reading the request…
t51c96267 = Anfrage wird gelesen…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = Der Body der Anfrage wurde nicht behalten, daher kann ihre Query nicht gezeigt werden.

# An operation without a name
t38c4d30b = Eine Operation ohne Namen

# {n} of {count}: {name}
t53045460 = { $n } von { $count }: { $name }

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Eine Persisted Query: Der Server kennt ihren Text, die Anfrage schickt nur ihren Hash.

# Variables
t7996f062 = Variablen

# Cut
t58e25863 = Ausschneiden

# Copy
t658f3664 = Kopieren

# Paste
tb34a0288 = Einfügen

# Select all
t6af21072 = Alles auswählen

# This flow is gone.
t3343999e = Dieser Flow ist nicht mehr vorhanden.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Dieser Host wird nicht entschlüsselt, daher ist nicht zu sehen, was er sendet.

# Still coming: it shows once it ends.
t42be95fb = Kommt noch: Er erscheint, sobald er endet.

# No body.
t31e4d044 = Kein Body.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Dieser Body wurde nicht gespeichert: Der Datenträger war fast voll, oder zu viel wartete darauf, geschrieben zu werden.

# This body was not kept.
te46616b5 = Dieser Body wurde nicht gespeichert.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Dieser Body ist nicht mehr vorhanden: Er wurde mit den ältesten Flows verworfen oder kann nicht gelesen werden.

# Cannot open this body: {e}
t620a039b = Dieser Body kann nicht geöffnet werden: { $e }

# Inspector
t4def410e = Inspektor

# Choose a flow to see it here
tc7be91e8 = Wählen Sie einen Flow, um ihn hier zu sehen

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Nicht entschlüsselt. Neue Verbindungen zu diesem Host werden ungelesen durchgeleitet.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Wenn eine App mit diesem Host fehlschlägt, nutzt sie möglicherweise Certificate Pinning.

# Make a rule
t0ac0e062 = Regel erstellen

# Decrypt this host
tb7367e2c = Diesen Host entschlüsseln

# Do not decrypt this host
t5fa113f6 = Diesen Host nicht entschlüsseln

# Request and response
t04e060e6 = Anfrage und Antwort

# Request and response one at a time
t70397119 = Anfrage und Antwort einzeln

# Request and response side by side
t3d35ce69 = Anfrage und Antwort nebeneinander

# Request above the response
t83c63123 = Anfrage über der Antwort

# Table
td70ceb7f = Tabelle

# Pretty
t7e09470d = Formatiert

# Raw
ta689aa91 = Roh

# Picture · {size}
te97ed633 = Bild · { $size }

# Messages {count}
te61b841a = Nachrichten { $count }

# Headers {headers}
t04b0146d = Header { $headers }

# Cookies {cookies}
td58eb84b = Cookies { $cookies }

# Query {query}
t4756f9f5 = Query { $query }

# Form {form}
tfa6e7203 = Formular { $form }

# Connection
t09ea4ff9 = Verbindung

# Save body
te2c44424 = Body speichern

# Send again
tf5910557 = Erneut senden

# Edit and send
tf837e542 = Bearbeiten und senden

# Copied
t8dc21305 = Kopiert

# Copy as cURL
t866d1e3e = Als cURL kopieren

# Click another flow…
t59253578 = Anderen Flow anklicken…

# Compare with…
t58dfe040 = Vergleichen mit…

# Compare with the call before
tef15286c = Mit dem vorigen Aufruf vergleichen

# Copy as…
t9c1ad448 = Kopieren als…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Dieses Bild kann nicht dargestellt werden. Speichern Sie den Body, um es anzusehen.

# Opening…
tc40f2321 = Wird geöffnet…

# No response
t8c2c37b1 = Keine Antwort

# Tunnel, not decrypted
tce693254 = Tunnel, nicht entschlüsselt

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# RPC: {method}
tf8162891 = RPC: { $method }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Über Verbindung { $id }, zuvor aufgebaut: Für den Verbindungsaufbau wurde keine Zeit benötigt.

# {app}, pid {pid}
t627147b4 = { $app }, PID { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Nicht gefunden: Er wurde möglicherweise beendet oder gehört einem anderen Benutzer

# On another device
t2b964a37 = Auf einem anderen Gerät

# CLIENT
t6d3cd99e = CLIENT

# Device
tbdcbb413 = Gerät

# Program
tc4253b6b = Programm

# Address
t700560f3 = Adresse

# SERVER
te3b27612 = SERVER

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = Die Daten dieser Verbindung werden nicht mehr gespeichert: Sie wurden mit älteren Flows verworfen.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Nicht entschlüsselt: tracewing hat sie ungelesen weitergeleitet.

# No connection was made.
tf348eb92 = Es wurde keine Verbindung aufgebaut.

# None: it was answered without one.
tac71808e = Keine: Die Anfrage wurde ohne Verbindung beantwortet.

# {id}, given up during TLS
t0ec66a8c = { $id }, während TLS abgebrochen

# {id}, made for this flow
t7fe32cfc = { $id }, für diesen Flow aufgebaut

# {id}, made before and used again
ta904c078 = { $id }, zuvor aufgebaut und wiederverwendet

# Server
t6d7302f2 = Server

# None: plain HTTP
t05606349 = Keine: unverschlüsseltes HTTP

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = DER SERVER HAT KEINE ZERTIFIKATE VORGELEGT

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = WAS DER SERVER VORGELEGT HAT, SEIN EIGENES ZUERST

# NO CERTIFICATES
te6f31dce = KEINE ZERTIFIKATE

# CERTIFICATE
t1c902790 = ZERTIFIKAT

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
        [one] { $n } ZERTIFIKAT, DAS DES SERVERS ZUERST
       *[other] { $n } ZERTIFIKATE, DAS DES SERVERS ZUERST
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing wollte mit diesem Server nicht fortfahren: { $error }. Wenn die App ihm auf andere Weise vertraut, etwa über eine Firmen-CA, wählen Sie „Diesen Host nicht entschlüsseln“, dann prüft die App den Server selbst.

# Cipher
t5b3bed6a = Chiffre

# none
tada7afdb = keine

# not valid until {when}
t0a14b2d8 = erst gültig ab { $when }

# expired {when}
t932538c2 = abgelaufen { $when }

# expires {when}
t679bcb51 = läuft { $when } ab

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
        [one] { $names } und { $n } weiterer
       *[other] { $names } und { $n } weitere
    }

# Issued by {issuer}
t15a1ce5a = Ausgestellt von { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Gültig von { $from } bis { $to } · { $when }

# Names {names}
tda75e459 = Namen { $names }

# Serial {serial}
td1f0d98b = Seriennummer { $serial }

# These headers were not kept.
tb8739fec = Diese Header wurden nicht gespeichert.

# No headers.
tf3dacb5e = Keine Header.

# Trailers, sent after the body
t3447e9de = Trailer, nach dem Body gesendet

# File {filename} · {kind} · {size}
t6dbd239f = Datei { $filename } · { $kind } · { $size }

# no type
t902a6ece = kein Typ

# No cookies were sent.
t58e85ab2 = Es wurden keine Cookies gesendet.

# No cookies were set.
tf57dbabc = Es wurden keine Cookies gesetzt.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · Signatur nicht geprüft

# JWT · signature not checked
t05f8b533 = JWT · Signatur nicht geprüft

# Header
t114f9980 = Header

# Payload
ta0639745 = Payload

# Expires
tc7aa0523 = Läuft ab

# Issued
tb1835ecc = Ausgestellt

# Not before
td10dacd9 = Nicht vor

# in {amount}
tfda57c3e = in { $amount }

# {amount} ago
t5f968014 = vor { $amount }

# Light
t8ad2f8cf = Hell

# Dark
t45570065 = Dunkel

# System
t8f3184bc = System

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
        [one] in { $count } Chunk gestreamt
       *[other] in { $count } Chunks gestreamt
    }

# stopped: {stop}
tfa48fb30 = gestoppt: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = Tokens: { $input } Eingabe · { $output } Ausgabe

# Reading the call…
t33b52a3c = Aufruf wird gelesen…

# Answer
tf94ea0ff = Antwort

# Thinking: {thinking}
t737ece33 = Denken: { $thinking }

# Calls {name}{id}
t017882e4 = Ruft { $name }{ $id } auf

# Nothing yet.
td9b71d16 = Noch nichts.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
        [one] { $text }… und { $count } weiteres Zeichen
       *[other] { $text }… und { $count } weitere Zeichen
    }

# Copy the answer
t69975ed7 = Antwort kopieren

# Asked
t4e3af9c3 = Gefragt

# Tools offered: {tools}
tb871bcf4 = Angebotene Tools: { $tools }

# Not kept
tba30b5fa = Nicht gespeichert

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Diese Nachricht wurde nicht gespeichert: Zu viel wartete, oder der Datenträger war fast voll.

# This message is empty.
tdb0ec01f = Diese Nachricht ist leer.

# No messages yet
t51e7624c = Noch keine Nachrichten

# {said} kept, of {size}
t3558745a = { $said } von { $size } gespeichert

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } ist keine ganze Zahl

# Latency
t23a45ef9 = Latenz

# Down
ta4995035 = Download

# Up
t42f35940 = Upload

# Loss
t69db303a = Verlust

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Geben Sie eine Latenz, eine Geschwindigkeit oder einen Verlust an, um zu verlangsamen

# Latency, ms
tefe18ab9 = Latenz, ms

# Down, kbit/s
t08c4ae33 = Download, kbit/s

# Up, kbit/s
td55cfe9a = Upload, kbit/s

# Loss, %
tea4af439 = Verlust, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Nur diese Hosts, z. B. *.example.com; alle, wenn leer

# Not kept for next time: {e}
tdce4d67c = Nicht für das nächste Mal gespeichert: { $e }

# Slow network
td51ddfce = Langsames Netzwerk

# Off: traffic goes as fast as it can
tf0f34db0 = Aus: Der Traffic läuft so schnell wie möglich

# Custom
t9525b78e = Benutzerdefiniert

# Only these hosts
tfe16992b = Nur diese Hosts

# Apply
tca77bbdb = Anwenden

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Gilt ab der nächsten Anfrage. Die Latenz kommt zu jedem Round Trip hinzu; die Geschwindigkeit ist eine Leitung, die sich alle verlangsamten Flows teilen; ein verlorenes Stück wartet darauf, erneut gesendet zu werden, wie bei TCP. Andere Hosts werden nicht verlangsamt.

# Header line {line} has no ":"
t891b076d = Headerzeile { $line } hat kein „:“

# Header line {line} has no name
te3247b96 = Headerzeile { $line } hat keinen Namen

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
        [one] { $n } Byte, das kein Text ist: Es wird unverändert weitergeleitet.
       *[other] { $n } Bytes, die kein Text sind: Sie werden unverändert weitergeleitet.
    }

# Nothing is paused
t8cf68b4c = Nichts ist angehalten

# "{status}" is not a status
t40a529af = „{ $status }“ ist kein Status

# REQUEST
t1f3f6dd2 = ANFRAGE

# RESPONSE
tc85f77de = ANTWORT

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Nichts ist angehalten. Fügen Sie unter Regeln eine Regel zum Anhalten hinzu, um Traffic hier anzuhalten.

# for {method} {url}
t093a86f6 = für { $method } { $url }

# The request, before the server gets it
tc840fed6 = Die Anfrage, bevor der Server sie erhält

# The response, before the app gets it
t6683d72a = Die Antwort, bevor die App sie erhält

# Send as it came
tb593e730 = Unverändert senden

# Drop
t108524ac = Verwerfen

# Paused
tde281feb = Angehalten

# Nothing is waiting.
tea613769 = Nichts wartet.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
        [one] { $n } wartet darauf, weitergesendet zu werden.
       *[other] { $n } warten darauf, weitergesendet zu werden.
    }

# Send all as they came
tdce25640 = Alle unverändert senden

# The list was not saved: {e}
tfceba380 = Die Liste wurde nicht gespeichert: { $e }

# {count} service: {names} | {count} services: {names}
tc1b8764c =
    { $n ->
        [one] { $count } Dienst: { $names }
       *[other] { $count } Dienste: { $names }
    }

# Add
t9dc3aa14 = Hinzufügen

# https://api.example.com or http://localhost:50051
t14f9744c = https://api.example.com oder http://localhost:50051

# Reading…
t6aad8485 = Wird gelesen…

# {count} file | {count} files
tde674bd8 =
    { $n ->
        [one] { $count } Datei
       *[other] { $count } Dateien
    }

# {count} message | {count} messages
t8a1cd2f6 =
    { $n ->
        [one] { $count } Nachricht
       *[other] { $count } Nachrichten
    }

# {count} service | {count} services
t51f71902 =
    { $n ->
        [one] { $count } Dienst
       *[other] { $count } Dienste
    }

# and {count} more problem | and {count} more problems
t591ce5ea =
    { $n ->
        [one] und { $count } weiteres Problem
       *[other] und { $count } weitere Probleme
    }

# Remove
t21a5901d = Entfernen

# Protobuf schemas
ta4401d1a = Protobuf-Schemas

# Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
te5f4f07c = Ordner mit .proto-Dateien oder mit protoc -o erstellte Descriptor-Sets, um Protobuf- und gRPC-Bodys mit Feldnamen zu zeigen. Ein gRPC-Aufruf findet seine Nachrichten über seinen Pfad, anderes Protobuf über den messageType in seinem Typ. Sie werden neu gelesen, wenn sie sich ändern.

# Add a folder…
tb5930e43 = Ordner hinzufügen…

# Add files…
ted08dbe5 = Dateien hinzufügen…

# Read again
t2f0dcc5d = Neu lesen

# Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.
tf6948a5a = Oder einen Server mit gRPC-Reflection nach seinen Dateien fragen. https:// geht durch tracewing; http:// ist gRPC ohne TLS, direkt zum Server.

# Asking…
tf9cf22fe = Wird gefragt…

# Fetch
t533a3183 = Abrufen

# Find a message
te6f78626 = Nachricht suchen

# Read as {message}
t87b6a380 = Als { $message } lesen

# Read as…
t1b90af95 = Lesen als…

# Requests to {path} on {host} are read as the message chosen.
t8826d766 = Anfragen an { $path } auf { $host } werden als die gewählte Nachricht gelesen.

# Answers from {path} on {host} are read as the message chosen.
tbbe012da = Antworten von { $path } auf { $host } werden als die gewählte Nachricht gelesen.

# Automatic (gRPC path or messageType)
t134c9a06 = Automatisch (gRPC-Pfad oder messageType)

# and {count} more: type to narrow them | and {count} more: type to narrow them
t70ff68f9 =
    { $n ->
        [one] und { $count } weitere: tippen, um einzugrenzen
       *[other] und { $count } weitere: tippen, um einzugrenzen
    }

# Write the server's address first
t4f68f91a = Zuerst die Adresse des Servers eingeben

# "{address}" is not a server's address
t8c4d03a6 = „{ $address }“ ist keine Serveradresse

# The server has no services to describe
tfa5b7ad8 = Der Server hat keine Dienste zu beschreiben

# The server sent no files for its services
tdca19bdc = Der Server hat keine Dateien für seine Dienste geschickt

# The server answered {status}: it may not offer gRPC reflection
t345fdb40 = Der Server antwortete { $status }: Er bietet vielleicht keine gRPC-Reflection

# The server answered {code}: it may not offer gRPC reflection. {message}
tb629329e = Der Server antwortete { $code }: Er bietet vielleicht keine gRPC-Reflection. { $message }

# The server answered nothing: it may not offer gRPC reflection
t6bbb6207 = Der Server antwortete nichts: Er bietet vielleicht keine gRPC-Reflection

# New connections go through the {kind} proxy {address}.
tb5f1ef96 = Neue Verbindungen gehen über den { $kind }-Proxy { $address }.

# Connections go straight to servers.
t4a64a08f = Verbindungen gehen direkt zu den Servern.

# relay.toml cannot be used, so connections go straight: {problem}
tbc6a2da3 = relay.toml lässt sich nicht verwenden, daher gehen Verbindungen direkt: { $problem }

# http://proxy.example.com:8080 or socks5://proxy.example.com:1080
tb945a053 = http://proxy.example.com:8080 oder socks5://proxy.example.com:1080

# Go through another proxy
tc67dfdb1 = Über einen anderen Proxy gehen

# Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080, and are kept as written in relay.toml. Empty, tracewing goes straight to servers.
t84509e00 = Lässt das Firmennetz nur über den eigenen Proxy nach draußen, geht tracewing über diesen zu den Servern. Apps senden ihren Verkehr weiter an tracewing. Benutzername und Passwort stehen in der Adresse, etwa http://name:passwort@proxy:8080, und werden so in relay.toml gespeichert. Leer geht tracewing direkt zu den Servern.

# Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host.
tcbc7d298 = Direkt erreichte Hosts, wie NO_PROXY sie schreibt: example.com ist er selbst und alles darunter, * jeder Host.

# Sent by the server, not asked for
td717c770 = Vom Server geschickt, nicht angefordert

# any
t2c29f04d = beliebig

# Failed
t5579c3d4 = Fehlgeschlagen

# The server answered 200: a call that fails says so in the body.
ta74e5f73 = Der Server hat mit 200 geantwortet: ein fehlgeschlagener Aufruf sagt es im Body.

# {count} item | {count} items
t534e422e =
    { $n ->
        [one] { $count } Element
       *[other] { $count } Elemente
    }

# {count} field | {count} fields
t0039810a =
    { $n ->
        [one] { $count } Feld
       *[other] { $count } Felder
    }

# and {count} more
tbdb1ff6a = und { $count } weitere

# succeeded
t6b166ff2 = erfolgreich

# failed: it was mined, and what it asked for did not happen
t46ea918c = fehlgeschlagen: sie kam in einen Block, und was sie verlangte geschah nicht

# none: it makes a contract
t188b58f6 = keine: er erstellt einen Vertrag

# {count} byte, which the signature does not say how to read | {count} bytes, which the signature does not say how to read
t36ffce30 =
    { $n ->
        [one] { $count } Byte, das die Signatur nicht zu lesen angibt
       *[other] { $count } Bytes, die die Signatur nicht zu lesen angibt
    }

# {count} byte that does not read as one | {count} bytes that do not read as one
ta7ee514c =
    { $n ->
        [one] { $count } Byte, das sich nicht als Transaktion liest
       *[other] { $count } Bytes, die sich nicht als Transaktion lesen
    }

# {count} place | {count} places
t0965779e =
    { $n ->
        [one] { $count } Stelle
       *[other] { $count } Stellen
    }

# {count} blob | {count} blobs
t95369f92 =
    { $n ->
        [one] { $count } Blob
       *[other] { $count } Blobs
    }

# 0, the kind from before EIP-2718
tc2fde48e = 0, die Art von vor EIP-2718

# The bodies were not kept, so what the call sent cannot be shown.
teaf455ca = Die Bodys wurden nicht behalten, daher kann nicht gezeigt werden, was der Aufruf gesendet hat.

# and {count} more calls
tf0ac7103 = und { $count } weitere Aufrufe

# {count} function | {count} functions
t9ffc3c7c =
    { $n ->
        [one] { $count } Funktion
       *[other] { $count } Funktionen
    }

# {count} event | {count} events
t4dd03992 =
    { $n ->
        [one] { $count } Ereignis
       *[other] { $count } Ereignisse
    }

# {count} error | {count} errors
t4fba4c4e =
    { $n ->
        [one] { $count } Fehler
       *[other] { $count } Fehler
    }

# Contract ABIs
t08fb00f2 = Vertrags-ABIs

# Folders or files of ABI JSON, as solc, Hardhat and Foundry write it, to name the functions, events and errors in a JSON-RPC call instead of showing four bytes. They are read again when they change. Nothing is sent anywhere: a signature is worked out from the ABI on this machine.
tfadffbd2 = Ordner oder Dateien mit ABI-JSON, wie solc, Hardhat und Foundry es schreiben, um die Funktionen, Ereignisse und Fehler in einem JSON-RPC-Aufruf zu benennen, statt vier Bytes zu zeigen. Sie werden erneut gelesen, wenn sie sich ändern. Nichts wird irgendwohin gesendet: eine Signatur wird aus der ABI auf diesem Rechner ermittelt.

# Block
tbd074002 = Blockieren

# Answer from a file
td8edab50 = Aus einer Datei antworten

# Send to another host
te3cd85d7 = An einen anderen Host senden

# Set a request header
tfad4aaaa = Anfrage-Header setzen

# Set a response header
tcbbc20d0 = Antwort-Header setzen

# Remove a request header
t5228dd84 = Anfrage-Header entfernen

# Remove a response header
teeb5735a = Antwort-Header entfernen

# Set a query parameter
t73d9419b = Query-Parameter setzen

# Remove a query parameter
t12633da9 = Query-Parameter entfernen

# Change the request body
t41101201 = Anfrage-Body ändern

# Change the response body
tdd259b73 = Antwort-Body ändern

# Change the status
t4bbdfba8 = Status ändern

# Pause the request
t40ecc39b = Anfrage anhalten

# Pause the response
t18a34799 = Antwort anhalten

# Status to answer with, like 403
t8f3ebb6c = Status für die Antwort, z. B. 403

# Full path of a file or folder
t9fb9ae4c = Vollständiger Pfad einer Datei oder eines Ordners

# Where to, like http://localhost:3000
tdfedd004 = Wohin, z. B. http://localhost:3000

# Header's name
t80793085 = Name des Headers

# Its value
t43968778 = Sein Wert

# Parameter's name
t313c8e35 = Name des Parameters

# Text to find, or empty for the whole body
t8255fc87 = Zu suchender Text, oder leer für den ganzen Body

# What to put in its place
t03671fd5 = Was an seine Stelle kommt

# Status the client gets, like 500
t5e4d3088 = Status, den der Client bekommt, z. B. 500

# "{first}" is not a status
td43128d3 = „{ $first }“ ist kein Status

# Block with {status}
t1727e984 = Mit { $status } blockieren

# Answer from {path}
taefc08fa = Aus { $path } antworten

# Send to {to}
t5e256873 = An { $to } senden

# Request: {header}: {value}
taa230766 = Anfrage: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Antwort: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Anfrage: ohne { $header }

# Response: no {header}
t23960a2e = Antwort: ohne { $header }

# Query: {name}={value}
t0ee6c1c0 = Query: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Query: ohne { $name }

# Status {status}
tad048e53 = Status { $status }

# Request body: {replace}
te4856c36 = Anfrage-Body: { $replace }

# Response body: {replace}
t8600dc42 = Antwort-Body: { $replace }

# Request body: {find} → {replace}
taec471a9 = Anfrage-Body: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Antwort-Body: { $find } → { $replace }

# Name
t0fe07306 = Name

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, z. B. api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Methode, leer für alle

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
        [one] { $n } Regel in { $path } geschrieben
       *[other] { $n } Regeln in { $path } geschrieben
    }

# The rules were not written: {e}
te7ac257f = Die Regeln wurden nicht geschrieben: { $e }

# Import
t573e0af4 = Importieren

# {path} could not be read: {e}
t32a3ce94 = { $path } konnte nicht gelesen werden: { $e }

# Nothing was imported: {e}
tb9f7257a = Nichts wurde importiert: { $e }

# Edit
tc2c76cb1 = Bearbeiten

# Delete
t5797ea6a = Löschen

# Change this rule
t9160f393 = Diese Regel ändern

# A new rule
t55a722a0 = Eine neue Regel

# It waits under Paused until you send it on, changed or not.
tca566a57 = Sie wartet unter Angehalten, bis Sie sie weitersenden, geändert oder nicht.

# Save
t4d2d5d68 = Speichern

# Cancel
t35afca3b = Abbrechen

# Rules
t428b2d80 = Regeln

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Sie gelten ab der nächsten Anfrage, auch während die Aufzeichnung pausiert. Von oben nach unten: die erste Sperre, Datei oder der erste Host gewinnt, und jede andere passende Regel gilt, der Reihe nach. Eine Regel, die einen Body oder einen Status ändert, ändert, was der Server geschickt hat.

# Import rules…
t5edc205d = Regeln importieren…

# Export rules…
t6e09ca20 = Regeln exportieren…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
        [one] { $n } Regel hinzugefügt
       *[other] { $n } Regeln hinzugefügt
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
        [one] { $n } bereits vorhanden
       *[other] { $n } bereits vorhanden
    }

# left out: {why}
ta81e95e2 = ausgelassen: { $why }

# Copy all
t6aef5c37 = Alles kopieren

# Too large to copy all
t2fe8ff9f = Zu groß, um alles zu kopieren

# No flows
t6dc463eb = Keine Flows

# Name it to keep it
t52c1a402 = Benennen, um sie zu behalten

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · von einer anderen Version geschrieben, kann daher nicht geöffnet werden

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
        [one] { $count } Flow · { $size }
       *[other] { $count } Flows · { $size }
    }

# Recording now
t3cb3e6fc = Wird gerade aufgezeichnet

# Export HAR
t4d6458cc = HAR exportieren

# Delete for good?
ta7737f13 = Endgültig löschen?

# Sessions
tb69a8aec = Sitzungen

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Eine Sitzung mit Namen bleibt erhalten, bis Sie sie löschen. Von den anderen bleiben die neuesten { $kept }.

# Working…
td5507854 = Wird bearbeitet…

# Open a session file
tae8622fc = Sitzungsdatei öffnen

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, Fiddler .saz oder Charles .chlsj

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = Exportierte HAR-Dateien lassen Cookies, Tokens, Schlüssel und Passwörter weg, in Headern, URLs und Bodys

# Other devices
t9e7d8362 = Andere Geräte

# Let other devices use the proxy
t13e626b3 = Anderen Geräten die Nutzung des Proxys erlauben

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = Smartphones und andere Computer in Ihrem Netzwerk können dann ihren Traffic hierher senden. Das kann aber auch jeder andere darin: Lassen Sie dies in einem Netzwerk, dem Sie nicht vertrauen, ausgeschaltet.

# Servers
tcd0cf113 = Server

# Trust what this computer trusts
tfea2d91b = Den Zertifikaten dieses Computers vertrauen

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = Server mit einem Zertifikat einer auf diesem Computer installierten Zertifizierungsstelle, etwa einer firmeneigenen, werden so erreicht, wie die Browser dieses Computers sie erreichen. Wenn aus, wird nur öffentlichen Zertifizierungsstellen vertraut. Wird beim nächsten Start von tracewing wirksam.

# AI tools
t294d7ea4 = KI-Tools

# Let AI tools see tokens and cookies
te1d26d3c = KI-Tools Tokens und Cookies sehen lassen

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = KI-Tools lesen den aufgezeichneten Traffic über den MCP-Server von tracewing. Wenn aus, werden die Werte von Authorization, Cookie und Set-Cookie aus dem weggelassen, was sie erhalten. Gilt sofort.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Apps von HTTP/3 fernhalten

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = HTTP/3 läuft über UDP, das kein Proxy überträgt, daher umgeht eine App, die es nutzt, tracewing. Dies entfernt die HTTP/3-Angebote der Server aus ihren Antworten, und Apps bleiben bei HTTP/2. Flows zeigen die Angebote weiterhin so, wie die Server sie gesendet haben.

# Size of a session
ta20fa102 = Größe einer Sitzung

# Past this, the oldest flows are dropped.
td2914f46 = Darüber hinaus werden die ältesten Flows verworfen.

# Room to leave on the disk
t8674a3b3 = Freizuhaltender Speicherplatz

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Ist weniger frei, werden Flows weiterhin aufgelistet, aber Bodys nicht gespeichert.

# Earlier sessions to keep
td356194a = Frühere Sitzungen behalten

# Older ones are deleted when a new session starts.
t66556513 = Ältere werden gelöscht, wenn eine neue Sitzung beginnt.

# Bodies
tdb4d0409 = Bodys

# Most kept of one body
t777508dd = Maximal gespeichert von einem Body

# It still reaches the client whole.
t386fee28 = Beim Client kommt er trotzdem vollständig an.

# Most unpacked for viewing
tc3fe2efd = Maximal zur Ansicht entpackt

# Stops a small download that unpacks to many GB.
tf9c49515 = Verhindert, dass ein kleiner Download auf viele GB entpackt wird.

# Small body size
ta8996af1 = Größe kleiner Bodys

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Kleinere Bodys warten im Arbeitsspeicher und teilen sich eine Datei. Größere erhalten beim Eintreffen eine eigene Datei.

# Compress plain text on disk
tee503c2e = Klartext auf dem Datenträger komprimieren

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Braucht etwa halb so viel Platz und wird übersprungen, wenn der Traffic dafür zu schnell ist.

# Compression level
td386c9ef = Komprimierungsstufe

# 1 to 19. Higher is smaller and slower.
td42a987d = 1 bis 19. Höher ist kleiner und langsamer.

# Search
tc646a2c9 = Suche

# Search URLs
t0c3e0b3d = URLs durchsuchen

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Findet jeden Teil einer URL während der Eingabe. Kostet wenig.

# Search bodies
t8eec0293 = Bodys durchsuchen

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Findet Wörter in Bodys. Kann so viel Speicherplatz belegen wie die Bodys selbst.

# Searched part of each body
tab293b21 = Durchsuchter Teil jedes Bodys

# Words past this are not found. More takes more disk.
t0bf65f4a = Wörter dahinter werden nicht gefunden. Mehr belegt mehr Speicherplatz.

# Under heavy traffic
ta99888cb = Bei hohem Traffic

# Memory for bodies waiting to be written
t7d6ff799 = Arbeitsspeicher für Bodys, die auf das Schreiben warten

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Darüber hinaus werden Flows ohne ihre Bodys aufgelistet. Der Traffic wird nie zurückgehalten.

# Memory for large bodies waiting to be written
t07b9de08 = Arbeitsspeicher für große Bodys, die auf das Schreiben warten

# Past this, large bodies stop being kept.
tfe61a4f3 = Darüber hinaus werden große Bodys nicht mehr gespeichert.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: „{ $text }“ ist keine ganze Zahl

# {key}: must not be negative
t9c4debaa = { $key }: darf nicht negativ sein

# After a restart
t58dcea2d = Nach einem Neustart

# Every setting is back to its default.
t763785e7 = Alle Einstellungen sind wieder auf ihrem Standardwert.

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d =
    { $n ->
        [one] { $n } Änderung wird beim nächsten Start von tracewing wirksam.
       *[other] { $n } Änderungen werden beim nächsten Start von tracewing wirksam.
    }

# Settings
t4b058728 = Einstellungen

# Undo
t71fd4acf = Rückgängig

# Back to defaults
ta3162a13 = Standardwerte wiederherstellen

# Hosts not to decrypt
tb457f0b3 = Nicht zu entschlüsselnde Hosts

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = Einer pro Zeile: ein Host, oder *.example.com für ihn und alles darunter. Ihr Traffic wird ungelesen durchgeleitet. Diese Liste gilt sofort.

# Authorities trusted for servers
t0a308a8a = Für Server vertraute Zertifizierungsstellen

# tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in {folder}. They are read when tracewing starts.
t571051f9 = tracewing vertraut Servern, für die öffentliche Zertifizierungsstellen bürgen, und denen, für die die dieses Computers bürgen. Diesen vertraut es auch: der eigenen Stelle einer Firma oder einer eigenen, als .pem-Datei in { $folder }. Sie werden beim Start von tracewing gelesen.

# Left out: {why}
t4a567d82 = Ausgelassen: { $why }

# None yet.
t786bc275 = Noch keine.

# The folder has changed since tracewing started: restart it to use what it holds now.
t65f2d800 = Der Ordner hat sich seit dem Start von tracewing geändert: neu starten, um den jetzigen Inhalt zu nutzen.

# Open the folder
tc7d7e212 = Ordner öffnen

# Read it again
td72efd5e = Neu lesen

# Language
t9a73db9b = Sprache

# System ({system_name})
tb5c7a1d4 = System ({ $system_name })

# Applies at once.
tefb7a0f9 = Gilt sofort.

# Your browser
tb5a48dd2 = Ihr Browser

# Its private key is in the system keychain.
t62a47808 = Sein privater Schlüssel liegt im System-Schlüsselbund.

# Its private key is in ca.key, which only you can read.
td9c6165d = Sein privater Schlüssel liegt in ca.key, das nur Sie lesen können.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = Sein privater Schlüssel liegt in ca.key, das nur Sie lesen können, da der System-Schlüsselbund ihn nicht aufnehmen kann: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Öffnen Sie die Seite. Wenn der Browser vor dem Zertifikat warnt oder die Seite nicht findet, ist einer der obigen Schritte für diesen Browser nicht erledigt.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } läuft über tracewing und vertraut ihm.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } läuft über tracewing, hat die Seite aber ohne HTTPS geöffnet. Öffnen Sie sie erneut über https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = Auch ein Smartphone oder ein anderer Computer kann seinen Traffic hierher senden. Aktivieren Sie in den Einstellungen „Anderen Geräten die Nutzung des Proxys erlauben“.

# this computer's address, port {port}
t2188b3f4 = die Adresse dieses Computers, Port { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = Stellen Sie auf dem Gerät den Wi-Fi-Proxy auf { $addr } ein. Öffnen Sie dort dann http://{ $host }, laden Sie das Zertifikat und vertrauen Sie ihm. Die Flows des Geräts zeigen seine Adresse und lassen sich mit device: filtern. Benennen Sie es über einen davon, unter Verbindung.

# Looking for it…
t50ee7452 = Wird gesucht…

# Installing. The system may ask you to confirm.
ta41c38ec = Wird installiert. Das System fragt möglicherweise nach einer Bestätigung.

# Removing. The system may ask you to confirm.
t62b22726 = Wird entfernt. Das System fragt eventuell nach einer Bestätigung.

# This computer trusts it.
t27d6bacc = Dieser Computer vertraut ihm.

# Installed in: {places}.
tb5bcc244 = Installiert in: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Kommandozeilen-Tools vertrauen ihm, aber Browser haben eigene Zertifikatsspeicher. Solange es dort nicht enthalten ist, schlägt HTTPS über tracewing im Browser fehl.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing liest HTTPS mit einem Zertifikat, das nur für diesen Computer erstellt wurde. Solange ihm nicht vertraut wird, schlägt HTTPS über tracewing fehl.

# Trust the certificate
t26618699 = Dem Zertifikat vertrauen

# Install
tb50cbd40 = Installieren

# Also for command-line tools
t0db8faea = Auch für Kommandozeilen-Tools

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Das Zertifikat aus allen Speichern dieses Computers entfernen? HTTPS über tracewing schlägt dann fehl, bis es wieder installiert ist. tracewing behält es, um es später wieder zu installieren.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = Browser unter Linux brauchen certutil, damit man ihnen sagen kann, ihm zu vertrauen. Installieren Sie es, dann hier Installieren:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = Browser unter Linux brauchen certutil, damit man ihnen sagen kann, ihm zu vertrauen. Installieren Sie die NSS-Tools Ihrer Distribution, dann hier Installieren.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = Dieser Desktop hat keine Systemproxy-Einstellung, die tracewing ändern kann. Öffnen Sie unten einen Browser separat unter Browser, oder ein Terminal.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } hat keine Systemproxy-Einstellung, die tracewing ändern kann. Öffnen Sie unten einen Browser separat unter Browser, oder ein Terminal.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Apps, die die Systemeinstellung verwenden, laufen jetzt über { $addr }. Sie wird zurückgesetzt, wenn tracewing beendet wird oder abstürzt.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Leitet den Traffic dieses Computers über tracewing, für Apps, die die Systemeinstellung verwenden. Oder richten Sie eine einzelne App selbst auf { $addr }.

# Turn on the system proxy
t52b88441 = System-Proxy aktivieren

# Turn off
tcd03e04b = Ausschalten

# Turn on
t65a0c5a7 = Einschalten

# Check it in your browser
tbed6e83a = Im Browser prüfen

# Open the test page
te4e36f65 = Testseite öffnen

# Set up
t43c06abc = Einrichten

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Drei Schritte, um den HTTPS-Traffic dieses Computers zu sehen.

# See traffic
t980eee79 = Traffic ansehen

# the browsers' own stores
tddf6836c = die eigenen Speicher der Browser

# Trusts the certificate.
tc5bba858 = Vertraut dem Zertifikat.

# Does not trust the certificate yet.
tf2a0b8c5 = Vertraut dem Zertifikat noch nicht.

# Looking…
t284579ba = Wird gesucht…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Hat noch kein Profil. Einmal öffnen, dann erneut suchen.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 =
    { $n ->
        [one] { $holding } von { $n } Profil vertraut dem Zertifikat.
       *[other] { $holding } von { $n } Profilen vertrauen dem Zertifikat.
    }

# Has not opened the test page yet.
t67f9cb49 = Hat die Testseite noch nicht geöffnet.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Hat die Testseite über tracewing geöffnet, per HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Hat die Testseite über tracewing geöffnet, aber ohne HTTPS.

# Browsers
td580efd4 = Browser

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Jeder entscheidet selbst, wem er vertraut und wohin er seinen Verkehr schickt. Prüfen öffnet darin die Testseite. Separat öffnen startet ihn mit einem eigenen Profil, das über tracewing geht, ohne den Systemproxy anzurühren.

# Look again
t98416222 = Erneut suchen

# No browser found on this computer.
t0e46bb34 = Kein Browser auf diesem Computer gefunden.

# Check
t592c8de7 = Prüfen

# Open on its own
t0da7783c = Separat öffnen

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = UDP-Port 443 ist auf diesem Computer blockiert, daher weichen Apps von HTTP/3 auf HTTP/2 aus und laufen über tracewing. Das endet, wenn tracewing beendet wird oder abstürzt.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Manche Apps sprechen HTTP/3 über UDP und umgehen den Proxy. Wird UDP-Port 443 auf diesem Computer blockiert, weichen sie auf HTTP/2 über tracewing aus. Dazu ist ein Administratorpasswort nötig, und es endet, wenn tracewing beendet wird. Ein VPN, das UDP-Port 443 nutzt, funktioniert währenddessen nicht.

# Apps on HTTP/3
t2cfe5270 = Apps mit HTTP/3

# Stop blocking
t318358d8 = Nicht mehr blockieren

# Block HTTP/3
t88231e40 = HTTP/3 blockieren

# Or one app at a time
t91c63f1c = Oder eine App nach der anderen

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Ein Terminal, das über tracewing geht, während der Systemproxy und alle anderen Apps bleiben, wie sie sind. Für einen Browser tut Separat öffnen unter Browser dasselbe, mit einem eigenen Profil.

# Open a terminal
t4b629c72 = Terminal öffnen

# iPhone and iPad
ta550cc97 = iPhone und iPad

# Flutter apps
ta04ee397 = Flutter-Apps

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. Wählen Sie unter Einstellungen, WLAN das Netzwerk, dann Proxy konfigurieren, Manuell: Server { $server }, Port { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Scannen Sie den Code mit der Kamera oder öffnen Sie { $page } in Safari und laden Sie das Zertifikat. Erlauben Sie den Download.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. Wählen Sie unter Einstellungen, Allgemein, VPN und Geräteverwaltung das Profil tracewing CA und installieren Sie es.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. Aktivieren Sie unter Einstellungen, Allgemein, Info, Zertifikatsvertrauenseinstellungen das volle Vertrauen für tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Öffnen Sie https://{ $host } in Safari. Wenn dort steht, dass es funktioniert, laufen Apps, die die Systemeinstellungen verwenden, über tracewing.

# When you are done
t62466f08 = Wenn Sie fertig sind

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. In Einstellungen, WLAN das Netz wählen, dann Proxy konfigurieren: Aus. Bleibt er an, hat das Gerät in diesem Netz kein Internet mehr, sobald tracewing schließt.

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. In Einstellungen, Allgemein, VPN und Geräteverwaltung tracewing CA wählen und das Profil entfernen. Das volle Vertrauen geht mit ihm.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. Bearbeiten Sie unter Einstellungen, WLAN das Netzwerk und stellen Sie unter Erweitert den Proxy auf Manuell: Host { $host }, Port { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Scannen Sie den Code oder öffnen Sie { $page } im Browser und laden Sie das Zertifikat.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. Wählen Sie unter Einstellungen, Sicherheit, Verschlüsselung und Anmeldedaten (die Namen variieren je nach Hersteller) Zertifikat installieren, dann CA-Zertifikat, und wählen Sie tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Seit Android 7 vertrauen Apps vom Nutzer hinzugefügten Zertifikaten nur, wenn sie das angeben. Fügen Sie für einen Debug-Build Ihrer eigenen App res/xml/network_security_config.xml hinzu:

# and point to it from AndroidManifest.xml:
t02ad844b = und verweisen Sie in AndroidManifest.xml darauf:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Andere Apps lehnen das Zertifikat weiterhin ab: Ihr Traffic erscheint als fehlgeschlagene Flows.

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. In Einstellungen, WLAN das Netz bearbeiten und den Proxy wieder auf Keiner stellen. Bleibt er an, hat das Gerät in diesem Netz kein Internet mehr, sobald tracewing schließt.

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. In Einstellungen, Sicherheit, Verschlüsselung und Anmeldedaten, Nutzeranmeldedaten tracewing wählen und entfernen. tracewing.crt auch aus Downloads löschen.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = Der HttpClient von Dart, den http und Dio in Flutter verwenden, folgt nicht der Proxy-Einstellung des Smartphones und vertraut unter Android keinen vom Nutzer hinzugefügten Zertifikaten. Leiten Sie ihn in einem Debug-Build hierher:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback lässt jedes Zertifikat durch, deshalb ist es nur für Debug-Builds gedacht.

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = TracewingProxy und seine Zeile in main() vor einer Veröffentlichung entfernen, auch wenn sie außerhalb von Debug-Builds nichts tun. Am Telefon wurde nichts geändert.

# Open settings
t1336b4d6 = Einstellungen öffnen

# Phones and other devices
t2e3c591c = Smartphones und andere Geräte

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Ein Smartphone, ein Tablet oder ein anderer Computer in diesem Netzwerk kann unter { $proxy } über tracewing laufen. Der Code öffnet die Seite mit dem Zertifikat. Die Flows des Geräts zeigen seine Adresse, lassen sich mit device: filtern und über einen davon unter Verbindung benennen.

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = Andere Geräte erreichen tracewing unter einer der Adressen dieses Computers nicht: { $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f =
    { $n ->
        [one] { $n } Gerät ging über tracewing ({ $names }). Wenn Sie fertig sind, schalten Sie seinen WLAN-Proxy aus, wie unten: bleibt er an, hat es in diesem Netz kein Internet mehr, sobald tracewing schließt.
       *[other] { $n } Geräte gingen über tracewing ({ $names }). Wenn Sie fertig sind, schalten Sie ihren WLAN-Proxy aus, wie unten: bleibt er an, haben sie in diesem Netz kein Internet mehr, sobald tracewing schließt.
    }

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Eingerichtet. Das Zertifikat gehört bis zum Neustart des Geräts zu den Systemzertifikaten.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Eingerichtet. Das Zertifikat liegt in Downloads: Wählen Sie in den gerade geöffneten Einstellungen Verschlüsselung und Anmeldedaten, Zertifikat installieren, CA-Zertifikat, und wählen Sie tracewing.crt.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Apps, die nur den Systemzertifikaten vertrauen, lehnen es weiterhin ab.

# An emulator.
t14b6b86d = Ein Emulator.

# On USB.
t67dafc14 = Über USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Bestätigen Sie die Abfrage zum USB-Debugging auf dem Gerät und suchen Sie dann erneut.

# Still starting. Look again in a moment.
t13f68b64 = Startet noch. Suchen Sie gleich erneut.

# adb says it is {state}.
t2b7d71da = adb meldet den Status { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Stellt den Proxy auf einem Android-Emulator oder einem per USB verbundenen Smartphone mit aktiviertem USB-Debugging ein und installiert das Zertifikat. Der Traffic kommt über ADB, daher muss keines von beiden in diesem Netzwerk sein.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = adb wurde nicht gefunden. Installieren Sie Android Studio oder die Android Platform Tools und suchen Sie dann erneut.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Kein Gerät verbunden. Starten Sie einen Emulator oder schließen Sie ein Smartphone mit aktiviertem USB-Debugging an und suchen Sie dann erneut.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Richten Sie ein Gerät ein, damit es seinen Traffic hierher sendet. Rückgängig oder das Beenden von tracewing entfernt den Proxy wieder.

# Android emulators and USB devices
t379e433c = Android-Emulatoren und USB-Geräte

# Find devices
t54d4af39 = Geräte suchen

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = Der Systemproxy und die Sperre für HTTP/3 werden zurückgesetzt, wenn tracewing schließt oder abstürzt, ebenso ein per USB eingerichtetes Android-Gerät. Das Zertifikat bleibt vertrauenswürdig, bis Entfernen in Schritt 1 es herausnimmt. Ein von Hand eingerichtetes Telefon behält WLAN-Proxy und Zertifikat, bis sie dort entfernt werden: Die Anleitung jedes Geräts sagt wie. Andere Geräte erreichen den Proxy dieses Computers, bis das in den Einstellungen ausgeschaltet wird.

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code und andere KI-Tools können die von tracewing aufgezeichneten Flows über seinen MCP-Server auflisten und lesen. Tokens und Cookies bleiben außen vor, sofern die Einstellungen sie nicht zulassen. In Claude Code:

# Elsewhere, in the tool's MCP settings:
tec38833c = Anderswo, in den MCP-Einstellungen des Tools:

# Flows
t4db77db2 = Flows

# Choose the flow above or below
tecb3adec = Flow darüber oder darunter auswählen

# The first or the last flow
t7e278a00 = Ersten oder letzten Flow

# A page up or down
t23b1814b = Eine Seite nach oben oder unten

# Filter
tf4a9c097 = Filter

# Pause or resume recording
t6e21519a = Aufzeichnung pausieren oder fortsetzen

# Clear the list
tf2a55d1f = Liste leeren

# Compare the flow clicked with the chosen one
te42cd067 = Den angeklickten Flow mit dem gewählten vergleichen

# The chosen flow
te0a729de = Der ausgewählte Flow

# Its request
tb3970784 = Seine Anfrage

# Its response
t221fc168 = Seine Antwort

# Send it again
tef7d1944 = Erneut senden

# Edit it and send it
t6bc99e5c = Bearbeiten und senden

# Copy it as cURL
tc99a11a3 = Als cURL kopieren

# The part before or after the one shown
tdf4221d9 = Der Teil vor oder nach dem gezeigten

# Save the body shown
t20d9b7c0 = Den gezeigten Body speichern

# Select all of the part shown
tf9168168 = Alles im angezeigten Bereich auswählen

# Copy the selected text
t00b090d3 = Ausgewählten Text kopieren

# Send it on, as edited
t8c35fa5a = Weiterschicken, wie bearbeitet

# Send it on as it was
t16f7abb5 = Unverändert weiterschicken

# Drop it
t4e7f48b9 = Verwerfen

# The window
t2af18488 = Das Fenster

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Einrichten, Traffic, Regeln, Angehalten, Erstellen, Sitzungen, Einstellungen

# Show or hide the sidebar
t67adca04 = Seitenleiste ein- oder ausblenden

# Show or hide the inspector
ta5a7b9e9 = Inspektor ein- oder ausblenden

# The inspector beside the list or below it
t51e214f7 = Inspektor neben oder unter der Liste

# The next or the previous field
ta4b4bd5e = Das nächste oder vorige Feld

# These shortcuts
tbdf3b62b = Diese Tastenkürzel

# Close them
tf3a4b1ef = Schließen

# Keyboard shortcuts
t31eaddb7 = Tastenkürzel

# Close
tcd86acc3 = Schließen

# The certificate: {e}
t0235648c = Das Zertifikat: { $e }

# The terminal
tf542b2f4 = Das Terminal

# The system proxy: {e}
tc99726ce = Der System-Proxy: { $e }

# Blocking HTTP/3: {e}
t024ec871 = HTTP/3 blockieren: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx oder Text in Bodys

# A name, as iPhone test
t19f89214 = Ein Name, z. B. iPhone Test

# The search index: {e}
te3c971c9 = Der Suchindex: { $e }

# Click the flow to compare #{id} with.
tac1ec1d2 = Klicke auf den Flow, mit dem #{ $id } verglichen werden soll.

# An earlier session: {name}
t73446a1d = Eine frühere Sitzung: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Schreibgeschützt. Die Suche deckt mehr ab, sobald ihr Index fertig ist. Die Aufzeichnung läuft weiter.

# Read-only. Recording goes on.
te2f35d9e = Schreibgeschützt. Die Aufzeichnung läuft weiter.

# Back to live
t317c0453 = Zurück zur Live-Ansicht

# {shown} of {total}
tf2cc0563 = { $shown } von { $total }

# Recording
te75cf20c = Aufzeichnung

# System proxy off · {addr}
t7e813d25 = System-Proxy aus · { $addr }

# System proxy…
t29e7dcd0 = System-Proxy…

# System proxy on · {addr}
t319ad6b9 = System-Proxy an · { $addr }

# Apps
t53d91aad = Apps

# Devices
t27acf320 = Geräte

# All hosts
tfc7bfe37 = Alle Hosts

# No host has this in its name
t838c1d44 = Kein Host enthält dies im Namen

# Hidden
t3c2dfe59 = Ausgeblendet

# Hosts
tcf9aefb4 = Hosts

# Newest
t324b6d0b = Neueste

# No flow fits this filter
t7eb295f4 = Kein Flow passt zu diesem Filter

# Nothing yet
tbd00747c = Noch nichts

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Aktivieren Sie oben den System-Proxy oder richten Sie eine App auf { $addr }. Zum Ausprobieren: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Neben der Liste

# Below the list
t23658daf = Unter der Liste

# Drop it on a place · Esc puts it back
t0fa632ba = Auf einem Platz ablegen · Esc setzt es zurück

# Traffic
t4b589266 = Traffic

# Certificate trusted on this computer
t7b93f551 = Zertifikat auf diesem Computer vertrauenswürdig

# Checking the certificate…
tbf8942c3 = Zertifikat wird geprüft…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Zertifikat wird installiert; das System fragt möglicherweise nach einer Bestätigung…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Das Zertifikat wird entfernt; das System fragt eventuell nach einer Bestätigung…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS schlägt fehl, bis dem Zertifikat vertraut wird: einrichten

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml wurde nicht verwendet: { $problem }

# A setting was not saved: {problem}
tff1a5018 = Eine Einstellung wurde nicht gespeichert: { $problem }

# The proxy stopped: {failure}
t04dcff11 = Der Proxy wurde gestoppt: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = Der Datenträger ist fast voll: Bodys werden nicht gespeichert

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
        [one] { $count } Flow nicht aufgezeichnet
       *[other] { $count } Flows nicht aufgezeichnet
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
        [one] { $count } Flow ohne Bodys
       *[other] { $count } Flows ohne Bodys
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
        [one] { $count } ältester Flow verworfen, um unter { $size } zu bleiben
       *[other] { $count } älteste Flows verworfen, um unter { $size } zu bleiben
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
        [one] { $count } Flow
       *[other] { $count } Flows
    }

# Export as HAR
t95b6b766 = Als HAR exportieren

# Shortcuts {keys}
t4ab0ecb4 = Tastenkürzel { $keys }

# Columns
tea156fdc = Spalten
