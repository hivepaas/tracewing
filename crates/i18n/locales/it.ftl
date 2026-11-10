# tracewing in Italian. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {size} more
ta8dc6eab = … altri { $size }

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } riga
       *[other] { $size } · { $count } righe
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · lettura { $percent }%

# Appearance: {now}. Click for {next}.
t332cfa6d = Aspetto: { $now }. Fai clic per passare a { $next }.

# Method
t98a2c750 = Metodo

# Status
t005ef20f = Stato

# App
t81f51acc = App

# Host
t302af18f = Host

# Path
teb66e456 = Percorso

# Size
ta6478e7c = Dimensione

# Time
tdfe4e404 = Tempo

# Number
t61e9eac0 = Numero

# "{method}" is not a method
tacdcf5cb = "{ $method }" non è un metodo

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = il body della sua richiesta non è stato conservato per intero, quindi non può essere reinviata

# name: value, one a line
t4df37bea = nome: valore, uno per riga

# No body
tfa4c2902 = Nessun body

# No flow is chosen
t550b0a98 = Nessun flow selezionato

# This flow: {e}
t296762f8 = Questo flow: { $e }

# Request
t82454152 = Richiesta

# Headers
t334f5989 = Header

# Body
t428a1095 = Body

# Sending…
te946e7bf = Invio…

# Send
tf28e14cf = Invia

# New
t8b242db1 = Nuova

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … altri { $count } byte: il flow in Traffico li contiene tutti.

# Response
t55a982fe = Risposta

# Compose
t18049f93 = Componi

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Una richiesta tua, o una reinviata. Passa da tracewing, quindi si applicano le regole e compare anche in Traffico.

# {version} both sides
t961a5ea0 = { $version } su entrambi i lati

# {client} to the proxy, {server} to the server
t3d237646 = { $client } verso il proxy, { $server } verso il server

# Queued
t9e437c74 = In coda

# Connect
tdb89a899 = Connessione

# Waiting
tc9450e48 = Attesa

# Download
t6964bb19 = Download

# No file picker: {e}
t6935fbd3 = Nessun selettore di file: { $e }

# Writing the HAR…
t725a0e89 = Scrittura del file HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
        [one] { $n } flow scritto in { $path }
       *[other] { $n } flow scritti in { $path }
    }

# The HAR was not written: {e}
tef8ec569 = Il file HAR non è stato scritto: { $e }

# Open
t538b10e9 = Apri

# Reading {path}…
tc0e7c8f4 = Lettura di { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
        [one] , { $n } voce esclusa
       *[other] , { $n } voci escluse
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
        [one] { $n } flow letto da una { $format }
       *[other] { $n } flow letti da una { $format }
    }

# The file was not read: {e}
t7f49510f = Il file non è stato letto: { $e }

# it is no longer kept
ta7b4c380 = non è più conservato

# {size} saved to {path}
tc6932467 = { $size } salvati in { $path }

# The body was not saved: {e}
t189900b4 = Il body non è stato salvato: { $e }

# Fiddler session
t9f5c89a3 = sessione Fiddler

# Charles session
tb4f56f2b = sessione Charles

# All
t75afe364 = Tutti

# Errors
te9ed60a6 = Errori

# Media
td10fcaa3 = Media

# Rule: {rule}
teca2074f = Regola: { $rule }

# Events
t003420a4 = Eventi

# Not decrypted
t8090a1ba = Non decifrato

# open
td35ec4c9 = aperto

# Cut
t58e25863 = Taglia

# Copy
t658f3664 = Copia

# Paste
tb34a0288 = Incolla

# Select all
t6af21072 = Seleziona tutto

# This flow is gone.
t3343999e = Questo flow non c'è più.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Questo host non viene decifrato, quindi non si può vedere cosa invia.

# No body.
t31e4d044 = Nessun body.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Questo body non è stato conservato: il disco era quasi pieno, o c'era troppo in attesa di scrittura.

# This body was not kept.
te46616b5 = Questo body non è stato conservato.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Questo body non c'è più: è stato scartato con i flow più vecchi, o non può essere letto.

# Cannot open this body: {e}
t620a039b = Impossibile aprire questo body: { $e }

# Inspector
t4def410e = Ispettore

# Choose a flow to see it here
tc7be91e8 = Scegli un flow per vederlo qui

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Non decifrato. Le nuove connessioni a questo host passano senza essere lette.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Se un'app non funziona con questo host, potrebbe usare il pinning dei certificati.

# Make a rule
t0ac0e062 = Crea una regola

# Decrypt this host
tb7367e2c = Decifra questo host

# Do not decrypt this host
t5fa113f6 = Non decifrare questo host

# Picture · {size}
te97ed633 = Immagine · { $size }

# Messages {count}
te61b841a = Messaggi { $count }

# Headers {headers}
t04b0146d = Header { $headers }

# Cookies {cookies}
td58eb84b = Cookie { $cookies }

# Query {query}
t4756f9f5 = Query { $query }

# Form {form}
tfa6e7203 = Modulo { $form }

# Connection
t09ea4ff9 = Connessione

# Save body
te2c44424 = Salva body

# Send again
tf5910557 = Reinvia

# Edit and send
tf837e542 = Modifica e invia

# Copied
t8dc21305 = Copiato

# Copy as cURL
t866d1e3e = Copia come cURL

# Copy as…
t9c1ad448 = Copia come…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Questa immagine non può essere mostrata. Salva il body per guardarla.

# Opening…
tc40f2321 = Apertura…

# No response
t8c2c37b1 = Nessuna risposta

# Tunnel, not decrypted
tce693254 = Tunnel, non decifrato

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Sulla connessione { $id }, aperta prima: nessun tempo speso per connettersi.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Non trovato: potrebbe essere terminato, o appartenere a un altro utente

# On another device
t2b964a37 = Su un altro dispositivo

# CLIENT
t6d3cd99e = CLIENT

# Device
tbdcbb413 = Dispositivo

# Program
tc4253b6b = Programma

# Address
t700560f3 = Indirizzo

# SERVER
te3b27612 = SERVER

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = I dati di questa connessione non sono più conservati: sono stati scartati con i flow più vecchi.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Non decifrata: tracewing l'ha inoltrata senza guardarla.

# No connection was made.
tf348eb92 = Non è stata aperta nessuna connessione.

# None: it was answered without one.
tac71808e = Nessuna: è stata data una risposta senza connessione.

# {id}, given up during TLS
t0ec66a8c = { $id }, abbandonata durante TLS

# {id}, made for this flow
t7fe32cfc = { $id }, aperta per questo flow

# {id}, made before and used again
ta904c078 = { $id }, aperta prima e riutilizzata

# Server
t6d7302f2 = Server

# None: plain HTTP
t05606349 = Nessuno: HTTP in chiaro

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = IL SERVER NON HA PRESENTATO CERTIFICATI

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = CIÒ CHE IL SERVER HA PRESENTATO, PRIMA IL SUO

# NO CERTIFICATES
te6f31dce = NESSUN CERTIFICATO

# CERTIFICATE
t1c902790 = CERTIFICATO

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
        [one] { $n } CERTIFICATO, PRIMA QUELLO DEL SERVER
       *[other] { $n } CERTIFICATI, PRIMA QUELLO DEL SERVER
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing non ha voluto proseguire con questo server: { $error }. Se l'app si fida del server in un altro modo, per esempio tramite una CA aziendale, scegli “Non decifrare questo host” e l'app verificherà il server da sola.

# Cipher
t5b3bed6a = Cifrario

# none
tada7afdb = nessuno

# not valid until {when}
t0a14b2d8 = non valido fino a { $when }

# expired {when}
t932538c2 = scaduto { $when }

# expires {when}
t679bcb51 = scade { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
        [one] { $names } e { $n } altro
       *[other] { $names } e altri { $n }
    }

# Issued by {issuer}
t15a1ce5a = Emesso da { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Valido dal { $from } al { $to } · { $when }

# Names {names}
tda75e459 = Nomi { $names }

# Serial {serial}
td1f0d98b = Numero di serie { $serial }

# These headers were not kept.
tb8739fec = Questi header non sono stati conservati.

# No headers.
tf3dacb5e = Nessun header.

# File {filename} · {kind} · {size}
t6dbd239f = File { $filename } · { $kind } · { $size }

# no type
t902a6ece = nessun tipo

# No cookies were sent.
t58e85ab2 = Non è stato inviato nessun cookie.

# No cookies were set.
tf57dbabc = Non è stato impostato nessun cookie.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · firma non verificata

# JWT · signature not checked
t05f8b533 = JWT · firma non verificata

# Header
t114f9980 = Header

# Payload
ta0639745 = Payload

# Expires
tc7aa0523 = Scade

# Issued
tb1835ecc = Emesso

# Not before
td10dacd9 = Non prima di

# in {amount}
tfda57c3e = tra { $amount }

# {amount} ago
t5f968014 = { $amount } fa

# Light
t8ad2f8cf = Chiaro

# Dark
t45570065 = Scuro

# System
t8f3184bc = Sistema

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
        [one] trasmesso in { $count } blocco
       *[other] trasmesso in { $count } blocchi
    }

# stopped: {stop}
tfa48fb30 = fermato: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = token: { $input } in input · { $output } in output

# Reading the call…
t33b52a3c = Lettura della chiamata…

# Answer
tf94ea0ff = Risposta

# Thinking: {thinking}
t737ece33 = Ragionamento: { $thinking }

# Calls {name}{id}
t017882e4 = Chiama { $name }{ $id }

# Nothing yet.
td9b71d16 = Ancora niente.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
        [one] { $text }… e { $count } altro carattere
       *[other] { $text }… e altri { $count } caratteri
    }

# Copy the answer
t69975ed7 = Copia la risposta

# Asked
t4e3af9c3 = Domanda

# Tools offered: {tools}
tb871bcf4 = Strumenti offerti: { $tools }

# Not kept
tba30b5fa = Non conservato

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Questo messaggio non è stato conservato: c'era troppo in attesa, o il disco era quasi pieno.

# This message is empty.
tdb0ec01f = Questo messaggio è vuoto.

# No messages yet
t51e7624c = Ancora nessun messaggio

# {said} kept, of {size}
t3558745a = { $said } conservati su { $size }

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } non è un numero intero

# Latency
t23a45ef9 = Latenza

# Down
ta4995035 = Download

# Up
t42f35940 = Upload

# Loss
t69db303a = Perdita

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Indica una latenza, una velocità o una perdita per rallentare il traffico

# Latency, ms
tefe18ab9 = Latenza, ms

# Down, kbit/s
t08c4ae33 = Download, kbit/s

# Up, kbit/s
td55cfe9a = Upload, kbit/s

# Loss, %
tea4af439 = Perdita, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Solo questi host, come *.example.com; tutti se vuoto

# Not kept for next time: {e}
tdce4d67c = Non salvato per la prossima volta: { $e }

# Slow network
td51ddfce = Rete lenta

# Off: traffic goes as fast as it can
tf0f34db0 = Disattivato: il traffico va alla massima velocità

# Custom
t9525b78e = Personalizzato

# Only these hosts
tfe16992b = Solo questi host

# Apply
tca77bbdb = Applica

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Si applica dalla prossima richiesta. La latenza si aggiunge a ogni andata e ritorno; la velocità è un collegamento condiviso da tutti i flow rallentati; un pezzo perso attende di essere reinviato, come fa TCP. Gli altri host non vengono rallentati.

# Header line {line} has no ":"
t891b076d = La riga di header { $line } non ha ":"

# Header line {line} has no name
te3247b96 = La riga di header { $line } non ha un nome

# URL
t7569633e = URL

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
        [one] { $n } byte che non è testo: viene inoltrato così come è arrivato.
       *[other] { $n } byte che non sono testo: vengono inoltrati così come sono arrivati.
    }

# Nothing is paused
t8cf68b4c = Niente in pausa

# "{status}" is not a status
t40a529af = "{ $status }" non è uno stato

# REQUEST
t1f3f6dd2 = RICHIESTA

# RESPONSE
tc85f77de = RISPOSTA

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Niente in pausa. Aggiungi una regola di pausa nella schermata Regole per fermare qui il traffico.

# for {method} {url}
t093a86f6 = per { $method } { $url }

# The request, before the server gets it
tc840fed6 = La richiesta, prima che la riceva il server

# The response, before the app gets it
t6683d72a = La risposta, prima che la riceva l'app

# Send as it came
tb593e730 = Invia così com'è

# Drop
t108524ac = Scarta

# Paused
tde281feb = In pausa

# Nothing is waiting.
tea613769 = Niente in attesa.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
        [one] { $n } in attesa di essere inoltrato.
       *[other] { $n } in attesa di essere inoltrati.
    }

# Send all as they came
tdce25640 = Invia tutto così com'è

# Block
tbd074002 = Blocca

# Answer from a file
td8edab50 = Rispondi da un file

# Send to another host
te3cd85d7 = Invia a un altro host

# Set a request header
tfad4aaaa = Imposta un header della richiesta

# Set a response header
tcbbc20d0 = Imposta un header della risposta

# Remove a request header
t5228dd84 = Rimuovi un header della richiesta

# Remove a response header
teeb5735a = Rimuovi un header della risposta

# Set a query parameter
t73d9419b = Imposta un parametro di query

# Remove a query parameter
t12633da9 = Rimuovi un parametro di query

# Change the request body
t41101201 = Cambia il body della richiesta

# Change the response body
tdd259b73 = Cambia il body della risposta

# Change the status
t4bbdfba8 = Cambia lo stato

# Pause the request
t40ecc39b = Metti in pausa la richiesta

# Pause the response
t18a34799 = Metti in pausa la risposta

# Status to answer with, like 403
t8f3ebb6c = Stato con cui rispondere, come 403

# Full path of a file or folder
t9fb9ae4c = Percorso completo di un file o di una cartella

# Where to, like http://localhost:3000
tdfedd004 = Destinazione, come http://localhost:3000

# Header's name
t80793085 = Nome dell'header

# Its value
t43968778 = Il suo valore

# Parameter's name
t313c8e35 = Nome del parametro

# Text to find, or empty for the whole body
t8255fc87 = Testo da cercare, o vuoto per tutto il body

# What to put in its place
t03671fd5 = Cosa mettere al suo posto

# Status the client gets, like 500
t5e4d3088 = Stato che riceve il client, come 500

# "{first}" is not a status
td43128d3 = "{ $first }" non è uno stato

# Block with {status}
t1727e984 = Blocca con { $status }

# Answer from {path}
taefc08fa = Rispondi da { $path }

# Send to {to}
t5e256873 = Invia a { $to }

# Request: {header}: {value}
taa230766 = Richiesta: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Risposta: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Richiesta: senza { $header }

# Response: no {header}
t23960a2e = Risposta: senza { $header }

# Query: {name}={value}
t0ee6c1c0 = Query: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Query: senza { $name }

# Status {status}
tad048e53 = Stato { $status }

# Request body: {replace}
te4856c36 = Body della richiesta: { $replace }

# Response body: {replace}
t8600dc42 = Body della risposta: { $replace }

# Request body: {find} → {replace}
taec471a9 = Body della richiesta: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Body della risposta: { $find } → { $replace }

# Name
t0fe07306 = Nome

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, come api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Metodo, o vuoto per tutti

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
        [one] { $n } regola scritta in { $path }
       *[other] { $n } regole scritte in { $path }
    }

# The rules were not written: {e}
te7ac257f = Le regole non sono state scritte: { $e }

# Import
t573e0af4 = Importa

# {path} could not be read: {e}
t32a3ce94 = Impossibile leggere { $path }: { $e }

# Nothing was imported: {e}
tb9f7257a = Non è stato importato niente: { $e }

# Edit
tc2c76cb1 = Modifica

# Delete
t5797ea6a = Elimina

# Change this rule
t9160f393 = Modifica questa regola

# A new rule
t55a722a0 = Una nuova regola

# It waits under Paused until you send it on, changed or not.
tca566a57 = Resta in attesa nella schermata In pausa finché non la inoltri, modificata o no.

# Save
t4d2d5d68 = Salva

# Add
t9dc3aa14 = Aggiungi

# Cancel
t35afca3b = Annulla

# Rules
t428b2d80 = Regole

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Valgono dalla richiesta successiva, anche con la registrazione in pausa. Dall'alto in basso: vince il primo blocco, file o host, e ogni altra regola adatta si applica, in ordine. Una regola che cambia un body o uno stato cambia ciò che il server ha inviato.

# Import rules…
t5edc205d = Importa regole…

# Export rules…
t6e09ca20 = Esporta regole…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
        [one] { $n } regola aggiunta
       *[other] { $n } regole aggiunte
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
        [one] { $n } già presente
       *[other] { $n } già presenti
    }

# left out: {why}
ta81e95e2 = esclusa: { $why }

# Copy all
t6aef5c37 = Copia tutto

# Too large to copy all
t2fe8ff9f = Troppo grande per copiare tutto

# No flows
t6dc463eb = Nessun flow

# Name it to keep it
t52c1a402 = Dalle un nome per conservarla

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · scritta da un'altra versione, quindi non può essere aperta

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
        [one] { $count } flow · { $size }
       *[other] { $count } flow · { $size }
    }

# Recording now
t3cb3e6fc = Registrazione in corso

# Export HAR
t4d6458cc = Esporta HAR

# Delete for good?
ta7737f13 = Eliminare definitivamente?

# Sessions
tb69a8aec = Sessioni

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Una sessione con un nome viene conservata finché non la elimini. Delle altre, restano le { $kept } più recenti.

# Working…
td5507854 = Elaborazione…

# Open a session file
tae8622fc = Apri un file di sessione

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, .saz di Fiddler o .chlsj di Charles

# Exported HAR files leave out tokens and cookies
td4a2f543 = I file HAR esportati escludono token e cookie

# Other devices
t9e7d8362 = Altri dispositivi

# Let other devices use the proxy
t13e626b3 = Consenti ad altri dispositivi di usare il proxy

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust. Takes effect when tracewing starts again.
t2e14229d = Telefoni e altri computer della tua rete possono così inviare qui il loro traffico. Ma anche chiunque altro sulla rete: lascia questa opzione disattivata su una rete di cui non ti fidi. Ha effetto al prossimo avvio di tracewing.

# Servers
tcd0cf113 = Server

# Trust what this computer trusts
tfea2d91b = Fidati di ciò di cui si fida questo computer

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = I server con un certificato di un'autorità installata su questo computer, come quella di un'azienda, vengono raggiunti come li raggiungono i browser di questo computer. Se disattivato, sono considerate attendibili solo le autorità pubbliche. Ha effetto al prossimo avvio di tracewing.

# AI tools
t294d7ea4 = Strumenti di IA

# Let AI tools see tokens and cookies
te1d26d3c = Consenti agli strumenti di IA di vedere token e cookie

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = Gli strumenti di IA leggono il traffico registrato tramite il server MCP di tracewing. Se disattivato, i valori di Authorization, Cookie e Set-Cookie vengono esclusi da ciò che ricevono. Si applica subito.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Tieni le app lontane da HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them. Takes effect when tracewing starts again.
t2aa7be0d = HTTP/3 viaggia su UDP, che nessun proxy trasporta, quindi un'app che lo usa aggira tracewing. Questa opzione rimuove dalle risposte dei server le loro offerte di HTTP/3, e le app restano su HTTP/2. I flow mostrano comunque le offerte come le hanno inviate i server. Ha effetto al prossimo avvio di tracewing.

# Size of a session
ta20fa102 = Dimensione di una sessione

# Past this, the oldest flows are dropped.
td2914f46 = Oltre questo limite, i flow più vecchi vengono scartati.

# Room to leave on the disk
t8674a3b3 = Spazio da lasciare libero sul disco

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Con meno spazio libero, i flow vengono ancora elencati ma i body non vengono conservati.

# Earlier sessions to keep
td356194a = Sessioni precedenti da conservare

# Older ones are deleted when a new session starts.
t66556513 = Quelle più vecchie vengono eliminate quando inizia una nuova sessione.

# Bodies
tdb4d0409 = Body

# Most kept of one body
t777508dd = Massimo conservato di un body

# It still reaches the client whole.
t386fee28 = Arriva comunque intero al client.

# Most unpacked for viewing
tc3fe2efd = Massimo decompresso per la visualizzazione

# Stops a small download that unpacks to many GB.
tf9c49515 = Impedisce che un piccolo download si decomprima in molti GB.

# Small body size
ta8996af1 = Dimensione dei body piccoli

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = I body più piccoli attendono in memoria e condividono un file. Quelli più grandi ricevono un file tutto loro man mano che arrivano.

# Compress plain text on disk
tee503c2e = Comprimi il testo semplice sul disco

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Occupa circa metà dello spazio, e viene saltata quando il traffico è troppo veloce.

# Compression level
td386c9ef = Livello di compressione

# 1 to 19. Higher is smaller and slower.
td42a987d = Da 1 a 19. Più alto è più piccolo e più lento.

# Search
tc646a2c9 = Ricerca

# Search URLs
t0c3e0b3d = Cerca negli URL

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Trova qualsiasi parte di un URL mentre scrivi. Costa poco.

# Search bodies
t8eec0293 = Cerca nei body

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Trova parole nei body. Può occupare tanto disco quanto i body stessi.

# Searched part of each body
tab293b21 = Parte cercata di ogni body

# Words past this are not found. More takes more disk.
t0bf65f4a = Le parole oltre questo limite non vengono trovate. Di più occupa più disco.

# Under heavy traffic
ta99888cb = Con traffico intenso

# Memory for bodies waiting to be written
t7d6ff799 = Memoria per i body in attesa di scrittura

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Oltre questo limite, i flow vengono elencati senza i loro body. Il traffico non viene mai trattenuto.

# Memory for large bodies waiting to be written
t07b9de08 = Memoria per i body grandi in attesa di scrittura

# Past this, large bodies stop being kept.
tfe61a4f3 = Oltre questo limite, i body grandi non vengono più conservati.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }" non è un numero intero

# {key}: must not be negative
t9c4debaa = { $key }: non deve essere negativo

# Saved. The limits apply the next time tracewing starts.
tc8bd2065 = Salvato. I limiti si applicano al prossimo avvio di tracewing.

# Settings
t4b058728 = Impostazioni

# Back to defaults
ta3162a13 = Ripristina i valori predefiniti

# Hosts not to decrypt
tb457f0b3 = Host da non decifrare

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies as soon as it is saved.
t7647c310 = Uno per riga: un host, oppure *.example.com per quell'host e tutto ciò che sta sotto. Il loro traffico passa senza essere letto. Questo elenco si applica appena viene salvato.

# Language
t9a73db9b = Lingua

# System ({system_name})
tb5c7a1d4 = Sistema ({ $system_name })

# Applies at once.
tefb7a0f9 = Si applica subito.

# Your browser
tb5a48dd2 = Il tuo browser

# Its private key is in the system keychain.
t62a47808 = La sua chiave privata è nel portachiavi di sistema.

# Its private key is in ca.key, which only you can read.
td9c6165d = La sua chiave privata è in ca.key, che solo tu puoi leggere.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = La sua chiave privata è in ca.key, che solo tu puoi leggere, perché il portachiavi di sistema non può accoglierla: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Apri la pagina. Se il browser mostra un avviso sul certificato, o non trova la pagina, uno dei passaggi qui sopra non è stato completato per quel browser.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } passa da tracewing e lo considera attendibile.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } passa da tracewing, ma ha aperto la pagina senza HTTPS. Riaprila con https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings, then start tracewing again.
t9f95b7b8 = Anche un telefono o un altro computer può inviare qui il suo traffico. Attiva “Consenti ad altri dispositivi di usare il proxy” in Impostazioni, poi riavvia tracewing.

# this computer's address, port {port}
t2188b3f4 = l'indirizzo di questo computer, porta { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = Sul dispositivo, imposta il proxy Wi-Fi su { $addr }. Poi apri lì http://{ $host }, scarica il certificato e consideralo attendibile. I suoi flow mostrano il suo indirizzo e si possono filtrare con device:. Dagli un nome da uno di essi, in Connessione.

# Looking for it…
t50ee7452 = Ricerca…

# Installing. The system may ask you to confirm.
ta41c38ec = Installazione. Il sistema potrebbe chiederti di confermare.

# Removing. The system may ask you to confirm.
t62b22726 = Rimozione in corso. Il sistema potrebbe chiedere conferma.

# This computer trusts it.
t27d6bacc = Questo computer lo considera attendibile.

# Installed in: {places}.
tb5bcc244 = Installato in: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Gli strumenti da riga di comando lo considerano attendibile, ma i browser hanno archivi propri. Finché non è in quelli, HTTPS tramite tracewing non funziona in un browser.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing legge HTTPS con un certificato creato solo per questo computer. Finché non è considerato attendibile, HTTPS tramite tracewing non funziona.

# Trust the certificate
t26618699 = Considera attendibile il certificato

# Install
tb50cbd40 = Installa

# Also for command-line tools
t0db8faea = Anche per gli strumenti da riga di comando

# Remove
t21a5901d = Rimuovi

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Rimuovere il certificato da tutti gli archivi di questo computer? HTTPS tramite tracewing poi fallisce finché non viene reinstallato. tracewing lo conserva per reinstallarlo più tardi.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = Su Linux i browser hanno bisogno di certutil per sapere che devono fidarsene. Installalo, poi Installa qui:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = Su Linux i browser hanno bisogno di certutil per sapere che devono fidarsene. Installa gli strumenti NSS della tua distribuzione, poi Installa qui.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = Questo desktop non ha un'impostazione del proxy di sistema che tracewing possa cambiare. Apri qui sotto un browser a parte da Browser, o un terminale.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } non ha un'impostazione del proxy di sistema che tracewing possa cambiare. Apri qui sotto un browser a parte da Browser, o un terminale.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Le app che usano l'impostazione di sistema ora passano da { $addr }. L'impostazione torna com'era quando tracewing si chiude, o se si arresta in modo anomalo.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Instrada il traffico di questo computer tramite tracewing, per le app che usano l'impostazione di sistema. Oppure indirizza tu una singola app su { $addr }.

# Turn on the system proxy
t52b88441 = Attiva il proxy di sistema

# Turn off
tcd03e04b = Disattiva

# Turn on
t65a0c5a7 = Attiva

# Check it in your browser
tbed6e83a = Verifica nel browser

# Open the test page
te4e36f65 = Apri la pagina di prova

# Set up
t43c06abc = Configura

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Tre passaggi per vedere il traffico HTTPS di questo computer.

# See traffic
t980eee79 = Vedi il traffico

# the browsers' own stores
tddf6836c = gli archivi propri dei browser

# Trusts the certificate.
tc5bba858 = Si fida del certificato.

# Does not trust the certificate yet.
tf2a0b8c5 = Non si fida ancora del certificato.

# Looking…
t284579ba = Ricerca…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Non ha ancora un profilo. Aprilo una volta, poi cerca di nuovo.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 =
    { $n ->
        [one] { $holding } su { $n } profilo si fida del certificato.
        [many] { $holding } su { $n } profili si fidano del certificato.
       *[other] { $holding } su { $n } profili si fidano del certificato.
    }

# Has not opened the test page yet.
t67f9cb49 = Non ha ancora aperto la pagina di prova.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Ha aperto la pagina di prova tramite tracewing, in HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Ha aperto la pagina di prova tramite tracewing, ma senza HTTPS.

# Browsers
td580efd4 = Browser

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Ognuno decide per conto suo di cosa fidarsi e dove mandare il traffico. Verifica vi apre la pagina di prova. Apri a parte lo avvia con un profilo suo che passa per tracewing, senza toccare il proxy di sistema.

# Look again
t98416222 = Cerca di nuovo

# No browser found on this computer.
t0e46bb34 = Nessun browser trovato su questo computer.

# Check
t592c8de7 = Verifica

# Open on its own
t0da7783c = Apri a parte

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = La porta UDP 443 è bloccata su questo computer, quindi le app ripiegano da HTTP/3 a HTTP/2 e passano da tracewing. Il blocco termina quando tracewing si chiude, o se si arresta in modo anomalo.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Alcune app usano HTTP/3, su UDP, e aggirano il proxy. Bloccare la porta UDP 443 su questo computer le fa ripiegare su HTTP/2 tramite tracewing. Serve la password di un amministratore, e il blocco termina quando tracewing si chiude. Nel frattempo, una VPN che usa la porta UDP 443 smette di funzionare.

# Apps on HTTP/3
t2cfe5270 = App su HTTP/3

# Stop blocking
t318358d8 = Smetti di bloccare

# Block HTTP/3
t88231e40 = Blocca HTTP/3

# Or one app at a time
t91c63f1c = Oppure un'app alla volta

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Un terminale che passa per tracewing, mentre il proxy di sistema e tutte le altre app restano come sono. Per un browser, Apri a parte in Browser fa lo stesso con un profilo suo.

# Open a terminal
t4b629c72 = Apri un terminale

# iPhone and iPad
ta550cc97 = iPhone e iPad

# Flutter apps
ta04ee397 = App Flutter

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. In Impostazioni, Wi-Fi, scegli la rete, poi Configura proxy, Manuale: server { $server }, porta { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Inquadra il codice con la fotocamera, o apri { $page } in Safari, e scarica il certificato. Consenti il download.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. In Impostazioni, Generali, VPN e gestione dispositivi, scegli il profilo tracewing CA e installalo.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. In Impostazioni, Generali, Info, Impostazioni attendibilità certificati, attiva l'attendibilità totale per tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Apri https://{ $host } in Safari. Se dice che funziona, le app che usano le impostazioni di sistema passano da tracewing.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. In Impostazioni, Wi-Fi, modifica la rete e, in Opzioni avanzate, imposta il proxy su Manuale: host { $host }, porta { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Inquadra il codice, o apri { $page } nel browser, e scarica il certificato.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. In Impostazioni, Sicurezza, Crittografia e credenziali (i nomi variano a seconda del produttore), scegli Installa un certificato, poi Certificato CA, e seleziona tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Da Android 7, le app considerano attendibili i certificati aggiunti dall'utente solo se lo dichiarano. Per una build di debug della tua app, aggiungi res/xml/network_security_config.xml:

# and point to it from AndroidManifest.xml:
t02ad844b = e fai riferimento a esso da AndroidManifest.xml:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Le altre app continuano a rifiutare il certificato: il loro traffico appare come flow non riusciti.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = L'HttpClient di Dart, usato da http e Dio in Flutter, non segue l'impostazione proxy del telefono e, su Android, non considera attendibili i certificati aggiunti dall'utente. In una build di debug, mandalo qui:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback lascia passare qualsiasi certificato, ed è per questo che va usato solo nelle build di debug.

# Open settings
t1336b4d6 = Apri le impostazioni

# Phones and other devices
t2e3c591c = Telefoni e altri dispositivi

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Un telefono, un tablet o un altro computer su questa rete può passare da tracewing su { $proxy }. Il codice apre la pagina con il suo certificato. I flow del dispositivo mostrano il suo indirizzo, si possono filtrare con device: e da uno di essi puoi dare un nome al dispositivo, in Connessione.

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Configurato. Il certificato è tra quelli di sistema fino al riavvio del dispositivo.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Configurato. Il certificato è in Download: nelle Impostazioni appena aperte, scegli Crittografia e credenziali, Installa un certificato, Certificato CA, e seleziona tracewing.crt.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Le app che considerano attendibili solo i certificati di sistema continueranno a rifiutarlo.

# An emulator.
t14b6b86d = Un emulatore.

# On USB.
t67dafc14 = Via USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Accetta la richiesta di debug USB sul dispositivo, poi cerca di nuovo.

# Still starting. Look again in a moment.
t13f68b64 = Ancora in avvio. Cerca di nuovo tra un momento.

# adb says it is {state}.
t2b7d71da = adb indica lo stato { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Imposta il proxy su un emulatore Android, o su un telefono collegato via USB con il debug USB attivo, e installa il certificato. Il traffico arriva tramite ADB, quindi nessuno dei due deve essere su questa rete.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = adb non trovato. Installa Android Studio, o gli strumenti della piattaforma Android, poi cerca di nuovo.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Nessun dispositivo collegato. Avvia un emulatore, o collega un telefono con il debug USB attivo, poi cerca di nuovo.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Configura un dispositivo perché invii qui il suo traffico. Ripristina, o la chiusura di tracewing, rimuove di nuovo il proxy dal dispositivo.

# Undo
t71fd4acf = Ripristina

# Android emulators and USB devices
t379e433c = Emulatori Android e dispositivi USB

# Find devices
t54d4af39 = Trova dispositivi

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code e altri strumenti di IA possono elencare e leggere i flow registrati da tracewing, tramite il suo server MCP. Token e cookie restano esclusi a meno che le Impostazioni non li consentano. In Claude Code:

# Elsewhere, in the tool's MCP settings:
tec38833c = Altrove, nelle impostazioni MCP dello strumento:

# Flows
t4db77db2 = Flow

# Choose the flow above or below
tecb3adec = Scegli il flow sopra o sotto

# The first or the last flow
t7e278a00 = Il primo o l'ultimo flow

# A page up or down
t23b1814b = Una pagina su o giù

# Filter
tf4a9c097 = Filtra

# Pause or resume recording
t6e21519a = Metti in pausa o riprendi la registrazione

# Clear the list
tf2a55d1f = Svuota l'elenco

# The chosen flow
te0a729de = Il flow selezionato

# Its request
tb3970784 = La sua richiesta

# Its response
t221fc168 = La sua risposta

# Send it again
tef7d1944 = Reinvialo

# Edit it and send it
t6bc99e5c = Modificalo e invialo

# Copy it as cURL
tc99a11a3 = Copialo come cURL

# Select all of the part shown
tf9168168 = Seleziona tutto nella parte mostrata

# Copy the selected text
t00b090d3 = Copia il testo selezionato

# The window
t2af18488 = La finestra

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Configura, Traffico, Regole, In pausa, Componi, Sessioni, Impostazioni

# Show or hide the sidebar
t67adca04 = Mostra o nascondi la barra laterale

# Show or hide the inspector
ta5a7b9e9 = Mostra o nascondi l'ispettore

# The inspector beside the list or below it
t51e214f7 = L'ispettore accanto all'elenco o sotto

# These shortcuts
tbdf3b62b = Queste scorciatoie

# Close them
tf3a4b1ef = Chiudile

# Keyboard shortcuts
t31eaddb7 = Scorciatoie da tastiera

# Close
tcd86acc3 = Chiudi

# The certificate: {e}
t0235648c = Il certificato: { $e }

# The terminal
tf542b2f4 = Il terminale

# The system proxy: {e}
tc99726ce = Il proxy di sistema: { $e }

# Blocking HTTP/3: {e}
t024ec871 = Blocco di HTTP/3: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx o testo nei body

# A name, as iPhone test
t19f89214 = Un nome, come iPhone di prova

# The search index: {e}
te3c971c9 = L'indice di ricerca: { $e }

# An earlier session: {name}
t73446a1d = Una sessione precedente: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Sola lettura. La ricerca copre di più man mano che il suo indice viene completato. La registrazione continua.

# Read-only. Recording goes on.
te2f35d9e = Sola lettura. La registrazione continua.

# Back to live
t317c0453 = Torna alla diretta

# {shown} of {total}
tf2cc0563 = { $shown } di { $total }

# Recording
te75cf20c = Registrazione

# System proxy off · {addr}
t7e813d25 = Proxy di sistema disattivato · { $addr }

# System proxy…
t29e7dcd0 = Proxy di sistema…

# System proxy on · {addr}
t319ad6b9 = Proxy di sistema attivo · { $addr }

# Apps
t53d91aad = App

# Devices
t27acf320 = Dispositivi

# Hosts
tcf9aefb4 = Host

# All hosts
tfc7bfe37 = Tutti gli host

# Hidden
t3c2dfe59 = Nascosti

# Newest
t324b6d0b = Più recenti

# No flow fits this filter
t7eb295f4 = Nessun flow corrisponde a questo filtro

# Nothing yet
tbd00747c = Ancora niente

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Attiva il proxy di sistema qui sopra, o indirizza un'app su { $addr }. Per provare: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Accanto all'elenco

# Below the list
t23658daf = Sotto l'elenco

# Drop it on a place · Esc puts it back
t0fa632ba = Rilascialo in una posizione · Esc lo rimette a posto

# Traffic
t4b589266 = Traffico

# Certificate trusted on this computer
t7b93f551 = Certificato considerato attendibile su questo computer

# Checking the certificate…
tbf8942c3 = Verifica del certificato…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Installazione del certificato; il sistema potrebbe chiederti di confermare…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Rimozione del certificato; il sistema potrebbe chiedere conferma…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS non funzionerà finché il certificato non sarà considerato attendibile: configura

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml non è stato usato: { $problem }

# The proxy stopped: {failure}
t04dcff11 = Il proxy si è fermato: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = Il disco è quasi pieno: i body non vengono conservati

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
        [one] { $count } flow non registrato
       *[other] { $count } flow non registrati
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
        [one] { $count } flow senza body
       *[other] { $count } flow senza body
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
        [one] { $count } flow più vecchio scartato per restare sotto { $size }
       *[other] { $count } flow più vecchi scartati per restare sotto { $size }
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
        [one] { $count } flow
       *[other] { $count } flow
    }

# Export as HAR
t95b6b766 = Esporta come HAR

# Shortcuts {keys}
t4ab0ecb4 = Scorciatoie { $keys }

# Columns
tea156fdc = Colonne
