# tracewing in Spanish. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 =
    { $n ->
        [one] … { $count } línea { $closer }
        [many] … { $count } líneas { $closer }
       *[other] … { $count } líneas { $closer }
    }

# … {size} more
ta8dc6eab = … { $size } más

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc =
    { $n ->
        [one] { $size } { $format } · { $count } línea de JSON
        [many] { $size } { $format } · { $count } líneas de JSON
       *[other] { $size } { $format } · { $count } líneas de JSON
    }

# The first {count} rows are shown.
t16aa1c6d = Solo se muestran las primeras { $count } filas.

# Unfold all
t3849d44e = Desplegar todo

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } línea
       *[other] { $size } · { $count } líneas
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · leyendo { $percent }%

# {call} could not be put in the body as calldata. Write it as hex, or let tracewing write the body.
t1d03e346 = No se pudo poner { $call } en el cuerpo como calldata. Escríbela como hex, o deja que tracewing escriba el cuerpo.

# That is not a call written out.
tc159f8f5 = Eso no es una llamada escrita.

# No call named {name} is known here. Write its types with it, as transfer(address,uint256)(…), or add its ABI under Contract ABIs in Settings.
t61568d43 = Aquí no se conoce ninguna llamada de nombre { $name }. Escribe sus tipos junto a ella, como transfer(address,uint256)(…), o añade su ABI en ABIs de contratos, en Ajustes.

# {name} is more than one call: {found}. Write the types with it, as {first}(…).
t48b75c8d = { $name } es más de una llamada: { $found }. Escribe los tipos junto a ella, como { $first }(…).

# {types} are not types a call can be written with.
t544218b9 = { $types } no son tipos con los que se pueda escribir una llamada.

# The values do not fit {signature}: {why}
t14015b69 = Los valores no encajan con { $signature }: { $why }

# Appearance: {now}. Click for {next}.
t332cfa6d = Apariencia: { $now }. Haz clic para pasar a { $next }.

# api.example.com, or *.example.com
tdc6588ac = api.example.com, o *.example.com

# Choose
t73c66160 = Elegir

# Choose the certificate's file.
t6c7ace6e = Elige el archivo del certificado.

# Say which host it is for.
taf39b2e1 = Indica para qué host es.

# Not used: {why}
t3a0298f3 = Sin usar: { $why }

# Remove
t21a5901d = Quitar

# Client certificates
te3ce8812 = Certificados de cliente

# For servers that ask the client for a certificate (mutual TLS): tracewing shows the one given for their host, in the app's place. A PEM certificate and its key without a passphrase; the files stay where they are. Each applies to the next connection.
tcd3f6864 = Para servidores que piden un certificado al cliente (TLS mutuo): tracewing muestra el indicado para su host, en lugar de la app. Un certificado PEM y su clave sin frase de paso; los archivos se quedan donde están. Cada uno se aplica a la siguiente conexión.

# None yet.
t786bc275 = Ninguna aún.

# Certificate…
t164f4d6c = Certificado…

# none chosen
t23b63bc7 = ninguno elegido

# Key…
t7aa250a8 = Clave…

# in the certificate's file
t13bc7052 = en el archivo del certificado

# Add
t9dc3aa14 = Añadir

# Method
t98a2c750 = Método

# Status
t005ef20f = Estado

# App
t81f51acc = App

# Host
t302af18f = Host

# Path
teb66e456 = Ruta

# Operation
t218e02a6 = Operación

# Size
ta6478e7c = Tamaño

# Time
tdfe4e404 = Tiempo

# Number
t61e9eac0 = Número

# A body was not kept, so the two cannot be compared.
t9d6abb2a = No se guardó un body, así que no se pueden comparar.

# {count} field differs | {count} fields differ
t6a8fff7f =
    { $n ->
        [one] { $count } campo es distinto
        [many] { $count } campos son distintos
       *[other] { $count } campos son distintos
    }

# … {count} line the same | … {count} lines the same
tcc37e7ea =
    { $n ->
        [one] … { $count } línea igual
        [many] … { $count } líneas iguales
       *[other] … { $count } líneas iguales
    }

# Nothing to compare.
t9f32d942 = Nada que comparar.

# URL
t7569633e = URL

# Protocol
t4a57ebd5 = Protocolo

# Type
td155d06d = Tipo

# Sizes
td1a4a19d = Tamaños

# Body
t428a1095 = Cuerpo

# Headers
t334f5989 = Encabezados

# Query
t9e89e783 = Consulta

# Request
t82454152 = Solicitud

# Response
t55a982fe = Respuesta

# Neither has headers kept.
tb43ef094 = Ninguno tiene cabeceras guardadas.

# Trailers
t7c44fa89 = Trailers

# Neither URL has a query.
t4d8203e0 = Ninguna URL tiene query.

# Comparing…
ta8565d31 = Comparando…

# No body
tfa4c2902 = Sin cuerpo

# {kind}, its first {size} compared
t1d894c39 = { $kind }, comparados sus primeros { $size }

# Neither has a body.
tf2c47725 = Ninguno tiene body.

# The bodies are the same.
t85152e01 = Los bodies son iguales.

# {count} field differs. | {count} fields differ.
t0f651f77 =
    { $n ->
        [one] { $count } campo es distinto.
        [many] { $count } campos son distintos.
       *[other] { $count } campos son distintos.
    }

# The first {count} are listed.
taae62a3e = Solo se listan los primeros { $count }.

# {count} line differs. | {count} lines differ.
tf57a3157 =
    { $n ->
        [one] { $count } línea es distinta.
        [many] { $count } líneas son distintas.
       *[other] { $count } líneas son distintas.
    }

# Fields
t3308bd9c = Campos

# Lines
t8a1a00dc = Líneas

# Compare
td57a0a54 = Comparar

# #{a} beside #{b}: what only #{a} has is red, what only #{b} has is green.
t71ce2de9 = #{ $a } junto a #{ $b }: lo que solo tiene #{ $a } va en rojo, lo que solo tiene #{ $b } va en verde.

# Swap
te519d36e = Intercambiar

# Back to Traffic
t644042d2 = Volver a Traffic

# "{method}" is not a method
tacdcf5cb = "{ $method }" no es un método

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = el cuerpo de su solicitud no se guardó completo, así que no se puede volver a enviar

# name: value, one a line
t4df37bea = nombre: valor, uno por línea

# No flow is chosen
t550b0a98 = Ningún flow seleccionado

# This flow: {e}
t296762f8 = Este flow: { $e }

# The body is not a {message}: {e}
t3220a10a = El cuerpo no es un { $message }: { $e }

# Protobuf {message}, written as JSON: it is sent as Protobuf.
t83b6cd6e = Protobuf { $message }, escrito como JSON: se envía como Protobuf.

# Written out in the body and sent as calldata: {calls}
tcaecdf04 = Escrito en el cuerpo y enviado como calldata: { $calls }

# {names} is the client's to work out, so what was written for it did not go. | {names} are the client's to work out, so what was written for them did not go.
te85cbcea =
    { $n ->
        [one] { $names } lo decide el cliente, así que lo que se escribió para él no se envió.
       *[other] { $names } los decide el cliente, así que lo que se escribió para ellos no se envió.
    }

# Stopped: no answer was read. What reached the server shows in Traffic.
t5a78825c = Detenido: no se leyó ninguna respuesta. Lo que llegó al servidor aparece en Traffic.

# Sending…
te946e7bf = Enviando…

# Send
tf28e14cf = Enviar

# Stop
t4b7f7705 = Detener

# New
t8b242db1 = Nueva

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … { $count } bytes más: el flow en Tráfico los tiene todos.

# Body · {message}
taa3cc3ff = Cuerpo · { $message }

# Compose
t18049f93 = Componer

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Una solicitud propia, o una enviada de nuevo. Pasa por tracewing, así que se aplican las reglas y también aparece en Tráfico.

# {version} both sides
t961a5ea0 = { $version } en ambos lados

# {client} to the proxy, {server} to the server
t3d237646 = { $client } hacia el proxy, { $server } hacia el servidor

# Queued
t9e437c74 = En cola

# Connect
tdb89a899 = Conexión

# Waiting
tc9450e48 = Espera

# Download
t6964bb19 = Descarga

# No file picker: {e}
t6935fbd3 = No hay selector de archivos: { $e }

# Writing the HAR…
t725a0e89 = Escribiendo el HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
        [one] { $n } flow escrito en { $path }
       *[other] { $n } flows escritos en { $path }
    }

# The HAR was not written: {e}
tef8ec569 = No se escribió el HAR: { $e }

# Open
t538b10e9 = Abrir

# Reading {path}…
tc0e7c8f4 = Leyendo { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
        [one] , { $n } entrada omitida
       *[other] , { $n } entradas omitidas
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
        [one] { $n } flow leído de una { $format }
       *[other] { $n } flows leídos de una { $format }
    }

# The file was not read: {e}
t7f49510f = No se leyó el archivo: { $e }

# it is no longer kept
ta7b4c380 = ya no se conserva

# {size} saved to {path}
tc6932467 = { $size } guardados en { $path }

# The body was not saved: {e}
t189900b4 = No se guardó el cuerpo: { $e }

# Fiddler session
t9f5c89a3 = sesión de Fiddler

# Charles session
tb4f56f2b = sesión de Charles

# All
t75afe364 = Todos

# Errors
te9ed60a6 = Errores

# Media
td10fcaa3 = Multimedia

# Rule: {rule}
teca2074f = Regla: { $rule }

# Events
t003420a4 = Eventos

# JSON lines
te8ce2db0 = Líneas JSON

# Not decrypted
t8090a1ba = No descifrado

# open
td35ec4c9 = abierto

# Reading the request…
t51c96267 = Leyendo la solicitud…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = El cuerpo de la solicitud no se guardó, así que su consulta no se puede mostrar.

# An operation without a name
t38c4d30b = Una operación sin nombre

# {n} of {count}: {name}
t53045460 = { $n } de { $count }: { $name }

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Una consulta persistida: el servidor guarda su texto y la solicitud solo envía su hash.

# Variables
t7996f062 = Variables

# Cut
t58e25863 = Cortar

# Copy
t658f3664 = Copiar

# Paste
tb34a0288 = Pegar

# Select all
t6af21072 = Seleccionar todo

# This flow is gone.
t3343999e = Este flow ya no existe.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Este host no se descifra, así que no se puede ver lo que envía.

# Still coming: it shows once it ends.
t42be95fb = Todavía está llegando: se muestra cuando termine.

# No body.
t31e4d044 = Sin cuerpo.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Este cuerpo no se guardó: el disco estaba casi lleno, o había demasiado esperando a escribirse.

# This body was not kept.
te46616b5 = Este cuerpo no se guardó.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Este cuerpo ya no existe: se descartó con los flows más antiguos, o no se puede leer.

# Cannot open this body: {e}
t620a039b = No se puede abrir este cuerpo: { $e }

# Inspector
t4def410e = Inspector

# Choose a flow to see it here
tc7be91e8 = Elige un flow para verlo aquí

# Not decrypted. New connections to this host go through unread.
ta05ec37a = No descifrado. Las conexiones nuevas a este host pasan sin leerse.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Si una app falla con este host, puede que use pinning de certificados.

# Make a rule
t0ac0e062 = Crear una regla

# Decrypt this host
tb7367e2c = Descifrar este host

# Do not decrypt this host
t5fa113f6 = No descifrar este host

# Request and response
t04e060e6 = Solicitud y respuesta

# Request and response one at a time
t70397119 = Solicitud y respuesta, una a la vez

# Request and response side by side
t3d35ce69 = Solicitud y respuesta lado a lado

# Request above the response
t83c63123 = Solicitud encima de la respuesta

# Table
td70ceb7f = Tabla

# Pretty
t7e09470d = Formateado

# Raw
ta689aa91 = Original

# Picture · {size}
te97ed633 = Imagen · { $size }

# Messages {count}
te61b841a = Mensajes { $count }

# Headers {headers}
t04b0146d = Encabezados { $headers }

# Cookies {cookies}
td58eb84b = Cookies { $cookies }

# Query {query}
t4756f9f5 = Consulta { $query }

# Form {form}
tfa6e7203 = Formulario { $form }

# Connection
t09ea4ff9 = Conexión

# Save body
te2c44424 = Guardar cuerpo

# Send again
tf5910557 = Volver a enviar

# Edit and send
tf837e542 = Editar y enviar

# Copied
t8dc21305 = Copiado

# Copy as cURL
t866d1e3e = Copiar como cURL

# Click another flow…
t59253578 = Haz clic en otro flujo…

# Compare with…
t58dfe040 = Comparar con…

# Compare with the call before
tef15286c = Comparar con la llamada anterior

# Copy as…
t9c1ad448 = Copiar como…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Esta imagen no se puede mostrar. Guarda el cuerpo para verla.

# Opening…
tc40f2321 = Abriendo…

# No response
t8c2c37b1 = Sin respuesta

# Tunnel, not decrypted
tce693254 = Túnel, no descifrado

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# RPC: {method}
tf8162891 = RPC: { $method }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Por la conexión { $id }, abierta antes: no se tardó nada en conectar.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = No encontrado: puede que haya terminado o que pertenezca a otro usuario

# On another device
t2b964a37 = En otro dispositivo

# CLIENT
t6d3cd99e = CLIENTE

# Device
tbdcbb413 = Dispositivo

# Program
tc4253b6b = Programa

# Address
t700560f3 = Dirección

# SERVER
te3b27612 = SERVIDOR

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = Los datos de esta conexión ya no se conservan: se descartaron con flows más antiguos.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = No descifrada: tracewing la dejó pasar sin mirarla.

# No connection was made.
tf348eb92 = No se abrió ninguna conexión.

# None: it was answered without one.
tac71808e = Ninguna: se respondió sin conexión.

# {id}, given up during TLS
t0ec66a8c = { $id }, abandonada durante TLS

# {id}, made for this flow
t7fe32cfc = { $id }, abierta para este flow

# {id}, made before and used again
ta904c078 = { $id }, abierta antes y reutilizada

# Server
t6d7302f2 = Servidor

# None: plain HTTP
t05606349 = Ninguno: HTTP sin cifrar

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = EL SERVIDOR NO MOSTRÓ CERTIFICADOS

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = LO QUE MOSTRÓ EL SERVIDOR, EL SUYO PRIMERO

# NO CERTIFICATES
te6f31dce = SIN CERTIFICADOS

# CERTIFICATE
t1c902790 = CERTIFICADO

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
        [one] { $n } CERTIFICADO, EL DEL SERVIDOR PRIMERO
       *[other] { $n } CERTIFICADOS, EL DEL SERVIDOR PRIMERO
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing no quiso continuar con este servidor: { $error }. Si la app confía en él de otra forma, como mediante una CA de empresa, elige “No descifrar este host” y la app comprobará el servidor por sí misma.

# Cipher
t5b3bed6a = Cifrado

# none
tada7afdb = ninguno

# not valid until {when}
t0a14b2d8 = no válido hasta { $when }

# expired {when}
t932538c2 = caducó { $when }

# expires {when}
t679bcb51 = caduca { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
        [one] { $names } y { $n } más
       *[other] { $names } y { $n } más
    }

# Issued by {issuer}
t15a1ce5a = Emitido por { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Válido desde { $from } hasta { $to } · { $when }

# Names {names}
tda75e459 = Nombres { $names }

# Serial {serial}
td1f0d98b = Número de serie { $serial }

# These headers were not kept.
tb8739fec = Estos encabezados no se guardaron.

# No headers.
tf3dacb5e = Sin encabezados.

# Trailers, sent after the body
t3447e9de = Trailers, enviados después del cuerpo

# File {filename} · {kind} · {size}
t6dbd239f = Archivo { $filename } · { $kind } · { $size }

# no type
t902a6ece = sin tipo

# No cookies were sent.
t58e85ab2 = No se enviaron cookies.

# No cookies were set.
tf57dbabc = No se establecieron cookies.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · firma no comprobada

# JWT · signature not checked
t05f8b533 = JWT · firma no comprobada

# Header
t114f9980 = Encabezado

# Payload
ta0639745 = Payload

# Light
t8ad2f8cf = Claro

# Dark
t45570065 = Oscuro

# System
t8f3184bc = Sistema

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
        [one] transmitido en { $count } fragmento
       *[other] transmitido en { $count } fragmentos
    }

# stopped: {stop}
tfa48fb30 = detenido: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = tokens: { $input } de entrada · { $output } de salida

# Reading the call…
t33b52a3c = Leyendo la llamada…

# Answer
tf94ea0ff = Respuesta

# Thinking: {thinking}
t737ece33 = Razonamiento: { $thinking }

# Calls {name}{id}
t017882e4 = Llama a { $name }{ $id }

# Nothing yet.
td9b71d16 = Nada todavía.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
        [one] { $text }… y { $count } carácter más
       *[other] { $text }… y { $count } caracteres más
    }

# Copy the answer
t69975ed7 = Copiar la respuesta

# Asked
t4e3af9c3 = Pregunta

# Tools offered: {tools}
tb871bcf4 = Herramientas ofrecidas: { $tools }

# Not kept
tba30b5fa = No guardado

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Este mensaje no se guardó: había demasiado esperando, o el disco estaba casi lleno.

# This message is empty.
tdb0ec01f = Este mensaje está vacío.

# No messages yet
t51e7624c = Aún no hay mensajes

# {said} kept, of {size}
t3558745a = { $said } guardados de { $size }

# The socket has closed.
t8d883a0f = El socket se ha cerrado.

# Too many are still waiting to go. Try again.
tbea55dfd = Demasiados esperan aún para salir. Inténtalo de nuevo.

# To the server
te1ef193a = Al servidor

# To the app
t1bcabce4 = A la app

# Layouts
t17809004 = Diseños

# Keep the panels as they are now under a name, to come back to them.
t2b6aeb69 = Guarda la disposición actual de los paneles con un nombre para volver a ella.

# Keep
t555cbe68 = Guardar

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } no es un número entero

# Latency
t23a45ef9 = Latencia

# Down
ta4995035 = Bajada

# Up
t42f35940 = Subida

# Loss
t69db303a = Pérdida

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Indica una latencia, una velocidad o una pérdida para ralentizar el tráfico

# Latency, ms
tefe18ab9 = Latencia, ms

# Down, kbit/s
t08c4ae33 = Bajada, kbit/s

# Up, kbit/s
td55cfe9a = Subida, kbit/s

# Loss, %
tea4af439 = Pérdida, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Solo estos hosts, como *.example.com; todos si está vacío

# Not kept for next time: {e}
tdce4d67c = No se guardó para la próxima vez: { $e }

# Slow network
td51ddfce = Red lenta

# Off: traffic goes as fast as it can
tf0f34db0 = Desactivado: el tráfico va tan rápido como puede

# Custom
t9525b78e = Personalizado

# Only these hosts
tfe16992b = Solo estos hosts

# Apply
tca77bbdb = Aplicar

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Se aplica desde la siguiente solicitud. La latencia se suma a cada ida y vuelta; la velocidad es un enlace que comparten todos los flows ralentizados; un fragmento perdido espera a reenviarse, como hace TCP. Los demás hosts no se ralentizan.

# Header line {line} has no ":"
t891b076d = La línea de encabezado { $line } no tiene ":"

# Header line {line} has no name
te3247b96 = La línea de encabezado { $line } no tiene nombre

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
        [one] { $n } byte que no es texto: se reenvía tal como llegó.
       *[other] { $n } bytes que no son texto: se reenvían tal como llegaron.
    }

# Nothing is paused
t8cf68b4c = No hay nada en pausa

# "{status}" is not a status
t40a529af = "{ $status }" no es un estado

# REQUEST
t1f3f6dd2 = SOLICITUD

# RESPONSE
tc85f77de = RESPUESTA

# MESSAGE TO THE SERVER
t270a6209 = MENSAJE AL SERVIDOR

# MESSAGE TO THE APP
t7d928b05 = MENSAJE A LA APP

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = No hay nada en pausa. Añade una regla de pausa en la pantalla Reglas para detener el tráfico aquí.

# {kind} on {url}
t1204e859 = { $kind } en { $url }

# for {method} {url}
t093a86f6 = para { $method } { $url }

# The request, before the server gets it
tc840fed6 = La solicitud, antes de que la reciba el servidor

# The response, before the app gets it
t6683d72a = La respuesta, antes de que la reciba la app

# A socket's message, before the server gets it
tda2ebc5d = Un mensaje del socket, antes de que lo reciba el servidor

# A socket's message, before the app gets it
tbce07e9d = Un mensaje del socket, antes de que lo reciba la app

# Message
tae0ed984 = Mensaje

# Send as it came
tb593e730 = Enviar tal como llegó

# Drop
t108524ac = Descartar

# Paused
tde281feb = En pausa

# Nothing is waiting.
tea613769 = No hay nada esperando.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
        [one] { $n } espera a ser reenviado.
       *[other] { $n } esperan a ser reenviados.
    }

# Send all as they came
tdce25640 = Enviar todo tal como llegó

# The list was not saved: {e}
tfceba380 = La lista no se guardó: { $e }

# {count} service: {names} | {count} services: {names}
tc1b8764c =
    { $n ->
        [one] { $count } servicio: { $names }
        [many] { $count } servicios: { $names }
       *[other] { $count } servicios: { $names }
    }

# https://api.example.com or http://localhost:50051
t14f9744c = https://api.example.com o http://localhost:50051

# Reading…
t6aad8485 = Leyendo…

# {count} file | {count} files
tde674bd8 =
    { $n ->
        [one] { $count } archivo
        [many] { $count } archivos
       *[other] { $count } archivos
    }

# {count} message | {count} messages
t8a1cd2f6 =
    { $n ->
        [one] { $count } mensaje
        [many] { $count } mensajes
       *[other] { $count } mensajes
    }

# {count} service | {count} services
t51f71902 =
    { $n ->
        [one] { $count } servicio
        [many] { $count } servicios
       *[other] { $count } servicios
    }

# and {count} more problem | and {count} more problems
t591ce5ea =
    { $n ->
        [one] y { $count } problema más
        [many] y { $count } problemas más
       *[other] y { $count } problemas más
    }

# Protobuf schemas
ta4401d1a = Esquemas Protobuf

# Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
te5f4f07c = Carpetas de archivos .proto, o descriptor sets hechos con protoc -o, para mostrar los cuerpos Protobuf y gRPC con el nombre de sus campos. Una llamada gRPC encuentra sus mensajes por su ruta; el resto de Protobuf, por el messageType que nombra su tipo. Se vuelven a leer cuando cambian.

# Add a folder…
tb5930e43 = Añadir una carpeta…

# Add files…
ted08dbe5 = Añadir archivos…

# Read again
t2f0dcc5d = Volver a leer

# Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.
tf6948a5a = O pide sus archivos a un servidor que ofrezca reflexión gRPC. https:// pasa por tracewing; http:// es gRPC sin TLS, directo al servidor.

# Asking…
tf9cf22fe = Preguntando…

# Fetch
t533a3183 = Obtener

# Find a message
te6f78626 = Buscar un mensaje

# Read as {message}
t87b6a380 = Leer como { $message }

# Read as…
t1b90af95 = Leer como…

# Requests to {path} on {host} are read as the message chosen.
t8826d766 = Las peticiones a { $path } en { $host } se leen como el mensaje elegido.

# Answers from {path} on {host} are read as the message chosen.
tbbe012da = Las respuestas de { $path } en { $host } se leen como el mensaje elegido.

# Automatic (gRPC path or messageType)
t134c9a06 = Automático (ruta gRPC o messageType)

# and {count} more: type to narrow them | and {count} more: type to narrow them
t70ff68f9 =
    { $n ->
        [one] y { $count } más: escribe para filtrar
        [many] y { $count } más: escribe para filtrar
       *[other] y { $count } más: escribe para filtrar
    }

# Nothing to read in this
t92a8694c = No hay nada que interpretar aquí

# Write the server's address first
t4f68f91a = Escribe primero la dirección del servidor

# "{address}" is not a server's address
t8c4d03a6 = "{ $address }" no es la dirección de un servidor

# The server has no services to describe
tfa5b7ad8 = El servidor no tiene servicios que describir

# The server sent no files for its services
tdca19bdc = El servidor no envió archivos para sus servicios

# The server answered {status}: it may not offer gRPC reflection
t345fdb40 = El servidor respondió { $status }: puede que no ofrezca reflexión gRPC

# The server answered {code}: it may not offer gRPC reflection. {message}
tb629329e = El servidor respondió { $code }: puede que no ofrezca reflexión gRPC. { $message }

# The server answered nothing: it may not offer gRPC reflection
t6bbb6207 = El servidor no respondió nada: puede que no ofrezca reflexión gRPC

# The password is kept in the system's keychain.
t3a48373c = La contraseña se guarda en el llavero del sistema.

# The password is kept in relay.toml, which only you can read.
tf84cb18b = La contraseña se guarda en relay.toml, que solo tú puedes leer.

# The password is kept in relay.toml, which only you can read: the keychain cannot be used ({why}).
t5a08b6d0 = La contraseña se guarda en relay.toml, que solo tú puedes leer: no se puede usar el llavero ({ $why }).

# New connections go through the {kind} proxy {address}.
tb5f1ef96 = Las conexiones nuevas pasan por el proxy { $kind } { $address }.

# Connections go straight to servers.
t4a64a08f = Las conexiones van directamente a los servidores.

# relay.toml cannot be used, so connections go straight: {problem}
tbc6a2da3 = No se puede usar relay.toml, así que las conexiones van directas: { $problem }

# http://proxy.example.com:8080 or socks5://proxy.example.com:1080
tb945a053 = http://proxy.example.com:8080 o socks5://proxy.example.com:1080

# Go through another proxy
tc67dfdb1 = Pasar por otro proxy

# Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080; the password is then kept in the system's keychain, not shown again. Empty, tracewing goes straight to servers.
t5c30a22f = Cuando la red de una empresa no deja salir nada salvo por su propio proxy, tracewing llega a los servidores a través de él. Las apps siguen enviando su tráfico a tracewing. El usuario y la contraseña van en la dirección, como http://nombre:contraseña@proxy:8080; la contraseña se guarda luego en el llavero del sistema y no se vuelve a mostrar. Vacío, tracewing va directo a los servidores.

# Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host.
tcbc7d298 = Hosts a los que se llega directamente, como los escribe NO_PROXY: example.com es él y todo lo que hay debajo, * es cualquier host.

# Sent by the server, not asked for
td717c770 = Enviado por el servidor, sin que se pidiera

# any
t2c29f04d = cualquiera

# Failed
t5579c3d4 = Falló

# The server answered 200: a call that fails says so in the body.
ta74e5f73 = El servidor respondió 200: una llamada que falla lo dice en el cuerpo.

# {count} item | {count} items
t534e422e =
    { $n ->
        [one] { $count } elemento
       *[other] { $count } elementos
    }

# {count} field | {count} fields
t0039810a =
    { $n ->
        [one] { $count } campo
       *[other] { $count } campos
    }

# and {count} more
tbdb1ff6a = y { $count } más

# {count} byte that does not read as one | {count} bytes that do not read as one
ta7ee514c =
    { $n ->
        [one] { $count } byte que no se lee como una transacción
       *[other] { $count } bytes que no se leen como una transacción
    }

# {program}, {count} bytes
t1ce6ad3b = { $program }, { $count } bytes

# succeeded
t6b166ff2 = con éxito

# failed: it was mined, and what it asked for did not happen
t46ea918c = falló: se minó, y lo que pedía no ocurrió

# none: it makes a contract
t188b58f6 = ninguna: crea un contrato

# {count} byte, which the signature does not say how to read | {count} bytes, which the signature does not say how to read
t36ffce30 =
    { $n ->
        [one] { $count } byte que la firma no dice cómo leer
       *[other] { $count } bytes que la firma no dice cómo leer
    }

# {count} place | {count} places
t0965779e =
    { $n ->
        [one] { $count } lugar
       *[other] { $count } lugares
    }

# {count} blob | {count} blobs
t95369f92 =
    { $n ->
        [one] { $count } blob
       *[other] { $count } blobs
    }

# 0, the kind from before EIP-2718
tc2fde48e = 0, el tipo de antes de EIP-2718

# The bodies were not kept, so what the call sent cannot be shown.
teaf455ca = No se guardaron los cuerpos, así que no se puede mostrar lo que envió la llamada.

# and {count} more calls
tf0ac7103 = y { $count } llamadas más

# {count} function | {count} functions
t9ffc3c7c =
    { $n ->
        [one] { $count } función
       *[other] { $count } funciones
    }

# {count} event | {count} events
t4dd03992 =
    { $n ->
        [one] { $count } evento
       *[other] { $count } eventos
    }

# {count} error | {count} errors
t4fba4c4e =
    { $n ->
        [one] { $count } error
       *[other] { $count } errores
    }

# Contract ABIs
t08fb00f2 = ABIs de contratos

# Folders or files of ABI JSON, as solc, Hardhat and Foundry write it, to name the functions, events and errors in a JSON-RPC call instead of showing four bytes. They are read again when they change. Nothing is sent anywhere: a signature is worked out from the ABI on this machine.
tfadffbd2 = Carpetas o archivos de ABI JSON, como los escriben solc, Hardhat y Foundry, para nombrar las funciones, eventos y errores de una llamada JSON-RPC en vez de mostrar cuatro bytes. Se vuelven a leer cuando cambian. No se envía nada a ningún sitio: la firma se calcula desde el ABI en esta máquina.

# Block
tbd074002 = Bloquear

# Answer from a file
td8edab50 = Responder desde un archivo

# Send to another host
te3cd85d7 = Enviar a otro host

# Set a request header
tfad4aaaa = Establecer un encabezado de solicitud

# Set a response header
tcbbc20d0 = Establecer un encabezado de respuesta

# Remove a request header
t5228dd84 = Quitar un encabezado de solicitud

# Remove a response header
teeb5735a = Quitar un encabezado de respuesta

# Set a query parameter
t73d9419b = Establecer un parámetro de consulta

# Remove a query parameter
t12633da9 = Quitar un parámetro de consulta

# Change the request body
t41101201 = Cambiar el cuerpo de la solicitud

# Change the response body
tdd259b73 = Cambiar el cuerpo de la respuesta

# Change the status
t4bbdfba8 = Cambiar el estado

# Pause the request
t40ecc39b = Pausar la solicitud

# Pause the response
t18a34799 = Pausar la respuesta

# Pause a socket's messages to the server
te5d065fe = Pausar los mensajes del socket al servidor

# Pause a socket's messages to the app
t482173e8 = Pausar los mensajes del socket a la app

# Status to answer with, like 403
t8f3ebb6c = Estado con el que responder, como 403

# Full path of a file or folder
t9fb9ae4c = Ruta completa de un archivo o carpeta

# Where to, like http://localhost:3000
tdfedd004 = Destino, como http://localhost:3000

# Header's name
t80793085 = Nombre del encabezado

# Its value
t43968778 = Su valor

# Parameter's name
t313c8e35 = Nombre del parámetro

# Text to find, or empty for the whole body
t8255fc87 = Texto a buscar, o vacío para todo el cuerpo

# What to put in its place
t03671fd5 = Lo que va en su lugar

# Status the client gets, like 500
t5e4d3088 = Estado que recibe el cliente, como 500

# "{first}" is not a status
td43128d3 = "{ $first }" no es un estado

# Block with {status}
t1727e984 = Bloquear con { $status }

# Answer from {path}
taefc08fa = Responder desde { $path }

# Send to {to}
t5e256873 = Enviar a { $to }

# Request: {header}: {value}
taa230766 = Solicitud: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Respuesta: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Solicitud: sin { $header }

# Response: no {header}
t23960a2e = Respuesta: sin { $header }

# Query: {name}={value}
t0ee6c1c0 = Consulta: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Consulta: sin { $name }

# Status {status}
tad048e53 = Estado { $status }

# Request body: {replace}
te4856c36 = Cuerpo de la solicitud: { $replace }

# Response body: {replace}
t8600dc42 = Cuerpo de la respuesta: { $replace }

# Request body: {find} → {replace}
taec471a9 = Cuerpo de la solicitud: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Cuerpo de la respuesta: { $find } → { $replace }

# Name
t0fe07306 = Nombre

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, como api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Método, o vacío para todos

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
        [one] { $n } regla escrita en { $path }
       *[other] { $n } reglas escritas en { $path }
    }

# The rules were not written: {e}
te7ac257f = No se escribieron las reglas: { $e }

# Import
t573e0af4 = Importar

# {path} could not be read: {e}
t32a3ce94 = No se pudo leer { $path }: { $e }

# Nothing was imported: {e}
tb9f7257a = No se importó nada: { $e }

# Edit
tc2c76cb1 = Editar

# Delete
t5797ea6a = Eliminar

# Change this rule
t9160f393 = Cambiar esta regla

# A new rule
t55a722a0 = Una regla nueva

# It waits under Paused until you send it on, changed or not.
tca566a57 = Espera en la pantalla En pausa hasta que la reenvíes, con cambios o sin ellos.

# Save
t4d2d5d68 = Guardar

# Cancel
t35afca3b = Cancelar

# Rules
t428b2d80 = Reglas

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Se aplican desde la siguiente solicitud, también con la grabación en pausa. De arriba abajo: el primer bloqueo, archivo o host gana, y toda otra regla que encaje se aplica, en orden. Una regla que cambia un cuerpo o un estado cambia lo que envió el servidor.

# Import rules…
t5edc205d = Importar reglas…

# Export rules…
t6e09ca20 = Exportar reglas…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
        [one] { $n } regla añadida
       *[other] { $n } reglas añadidas
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
        [one] { $n } ya estaba aquí
       *[other] { $n } ya estaban aquí
    }

# left out: {why}
ta81e95e2 = omitida: { $why }

# Copy all
t6aef5c37 = Copiar todo

# Too large to copy all
t2fe8ff9f = Demasiado grande para copiarlo todo

# No flows
t6dc463eb = Sin flows

# Name it to keep it
t52c1a402 = Ponle nombre para conservarla

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · escrita por otra versión, así que no se puede abrir

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
        [one] { $count } flow · { $size }
       *[other] { $count } flows · { $size }
    }

# Recording now
t3cb3e6fc = Grabando ahora

# Export HAR
t4d6458cc = Exportar HAR

# Delete for good?
ta7737f13 = ¿Eliminar definitivamente?

# Sessions
tb69a8aec = Sesiones

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Una sesión con nombre se conserva hasta que la elimines. De las demás, se quedan las { $kept } más recientes.

# Working…
td5507854 = Procesando…

# Open a session file
tae8622fc = Abrir un archivo de sesión

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, .saz de Fiddler o .chlsj de Charles

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = Los archivos HAR exportados omiten cookies, tokens, claves y contraseñas, en encabezados, URL y cuerpos

# Other devices
t9e7d8362 = Otros dispositivos

# Let other devices use the proxy
t13e626b3 = Permitir que otros dispositivos usen el proxy

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = Así, los teléfonos y otros equipos de tu red pueden enviar su tráfico aquí. También cualquier otra persona en ella: déjalo desactivado en una red en la que no confíes.

# Servers
tcd0cf113 = Servidores

# Trust what this computer trusts
tfea2d91b = Confiar en lo que confía este equipo

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = A los servidores con un certificado de una autoridad instalada en este equipo, como la de una empresa, se accede igual que lo hacen los navegadores de este equipo. Desactivado, solo se confía en las autoridades públicas. Se aplica al reiniciar tracewing.

# AI tools
t294d7ea4 = Herramientas de IA

# Let AI tools see tokens and cookies
te1d26d3c = Permitir que las herramientas de IA vean tokens y cookies

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = Las herramientas de IA leen el tráfico grabado a través del servidor MCP de tracewing. Desactivado, los valores de Authorization, Cookie y Set-Cookie se omiten de lo que reciben. Se aplica al instante.

# Let AI tools add rules and send requests
t663de1ba = Permitir que las herramientas de IA añadan reglas y envíen peticiones

# Through the MCP server, an AI tool may then add and remove rules, which change traffic as yours do, and send requests through tracewing, again or changed, which reach their servers. Off, it can only read. Applies at once.
t46d53486 = A través del servidor MCP, una herramienta de IA puede añadir y quitar reglas, que cambian el tráfico como las tuyas, y enviar peticiones a través de tracewing, de nuevo o cambiadas, que llegan a sus servidores. Desactivado, solo puede leer. Se aplica al instante.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Mantener las apps fuera de HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = HTTP/3 va sobre UDP, que ningún proxy transporta, así que una app que lo usa evita tracewing. Esto quita de las respuestas de los servidores sus ofertas de HTTP/3, y las apps se quedan en HTTP/2. Los flows siguen mostrando las ofertas tal como las enviaron los servidores.

# Size of a session
ta20fa102 = Tamaño de una sesión

# Past this, the oldest flows are dropped.
td2914f46 = Por encima de esto, se descartan los flows más antiguos.

# Room to leave on the disk
t8674a3b3 = Espacio libre que dejar en el disco

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Con menos espacio libre, los flows se siguen listando pero los cuerpos no se guardan.

# Earlier sessions to keep
td356194a = Sesiones anteriores que conservar

# Older ones are deleted when a new session starts.
t66556513 = Las más antiguas se eliminan al empezar una sesión nueva.

# Bodies
tdb4d0409 = Cuerpos

# Most kept of one body
t777508dd = Máximo guardado de un cuerpo

# It still reaches the client whole.
t386fee28 = Aun así, llega entero al cliente.

# Most unpacked for viewing
tc3fe2efd = Máximo descomprimido para ver

# Stops a small download that unpacks to many GB.
tf9c49515 = Evita que una descarga pequeña se descomprima en muchos GB.

# Small body size
ta8996af1 = Tamaño de cuerpo pequeño

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Los cuerpos más pequeños esperan en memoria y comparten un archivo. Los más grandes reciben un archivo propio a medida que llegan.

# Compress plain text on disk
tee503c2e = Comprimir el texto plano en disco

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Ocupa más o menos la mitad, y se omite cuando el tráfico es demasiado rápido.

# Compression level
td386c9ef = Nivel de compresión

# 1 to 19. Higher is smaller and slower.
td42a987d = De 1 a 19. Más alto es más pequeño y más lento.

# Search
tc646a2c9 = Búsqueda

# Search URLs
t0c3e0b3d = Buscar en URLs

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Encuentra cualquier parte de una URL mientras escribes. Cuesta poco.

# Search bodies
t8eec0293 = Buscar en cuerpos

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Encuentra palabras en los cuerpos. Puede ocupar tanto disco como los propios cuerpos.

# Searched part of each body
tab293b21 = Parte buscada de cada cuerpo

# Words past this are not found. More takes more disk.
t0bf65f4a = Las palabras más allá no se encuentran. Más ocupa más disco.

# Under heavy traffic
ta99888cb = Con tráfico intenso

# Memory for bodies waiting to be written
t7d6ff799 = Memoria para cuerpos pendientes de escribir

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Por encima de esto, los flows se listan sin sus cuerpos. El tráfico nunca se retiene.

# Memory for large bodies waiting to be written
t07b9de08 = Memoria para cuerpos grandes pendientes de escribir

# Past this, large bodies stop being kept.
tfe61a4f3 = Por encima de esto, los cuerpos grandes dejan de guardarse.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }" no es un número entero

# {key}: must not be negative
t9c4debaa = { $key }: no debe ser negativo

# After a restart
t58dcea2d = Tras reiniciar

# Every setting is back to its default.
t763785e7 = Todos los ajustes han vuelto a su valor predeterminado.

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d =
    { $n ->
        [one] { $n } cambio se aplica al reiniciar tracewing.
        [many] { $n } cambios se aplican al reiniciar tracewing.
       *[other] { $n } cambios se aplican al reiniciar tracewing.
    }

# Settings
t4b058728 = Ajustes

# Undo
t71fd4acf = Deshacer

# Back to defaults
ta3162a13 = Restablecer valores predeterminados

# Hosts not to decrypt
tb457f0b3 = Hosts que no descifrar

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = Uno por línea: un host, o *.example.com para él y todo lo que hay debajo. Su tráfico pasa sin leerse. Esta lista se aplica al instante.

# Authorities trusted for servers
t0a308a8a = Autoridades de confianza para servidores

# tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in {folder}. They are read when tracewing starts.
t571051f9 = tracewing confía en los servidores que avalan las autoridades públicas y las de este equipo. También confía en estas: la autoridad propia de una empresa, o una tuya, puesta como archivo .pem en { $folder }. Se leen al iniciar tracewing.

# Left out: {why}
t4a567d82 = Omitido: { $why }

# The folder has changed since tracewing started: restart it to use what it holds now.
t65f2d800 = La carpeta cambió desde que se inició tracewing: reinícialo para usar lo que contiene ahora.

# Open the folder
tc7d7e212 = Abrir la carpeta

# Read it again
td72efd5e = Volver a leer

# TLS keys for Wireshark
tcb4394af = Claves TLS para Wireshark

# Writes the secrets of each HTTPS connection, from apps to tracewing and from it to servers, to a file Wireshark reads: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Anyone with the file can read the traffic it covers. Off again when tracewing starts, unless SSLKEYLOGFILE names a file.
t2f92ac10 = Escribe los secretos de cada conexión HTTPS, de las apps a tracewing y de este a los servidores, en un archivo que Wireshark lee: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Cualquiera con el archivo puede leer el tráfico que cubre. Se apaga de nuevo al iniciar tracewing, salvo que SSLKEYLOGFILE indique un archivo.

# Cannot write to it: {why}
t956ecc4b = No se puede escribir en él: { $why }

# Copy the path
t095d064e = Copiar la ruta

# Language
t9a73db9b = Idioma

# System ({system_name})
tb5c7a1d4 = Sistema ({ $system_name })

# Applies at once.
tefb7a0f9 = Se aplica al instante.

# Your browser
tb5a48dd2 = Tu navegador

# Its private key is in the system keychain.
t62a47808 = Su clave privada está en el llavero del sistema.

# Its private key is in ca.key, which only you can read.
td9c6165d = Su clave privada está en ca.key, que solo tú puedes leer.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = Su clave privada está en ca.key, que solo tú puedes leer, ya que el llavero del sistema no la admite: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Abre la página. Si el navegador avisa sobre el certificado, o no encuentra la página, falta alguno de los pasos anteriores para ese navegador.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } pasa por tracewing y confía en él.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } pasa por tracewing, pero abrió la página sin HTTPS. Vuelve a abrirla con https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = Un teléfono u otro equipo también puede enviar su tráfico aquí. Activa “Permitir que otros dispositivos usen el proxy” en Ajustes.

# this computer's address, port {port}
t2188b3f4 = la dirección de este equipo, puerto { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = En el dispositivo, configura el proxy de la Wi-Fi como { $addr }. Después abre allí http://{ $host }, descarga el certificado y confía en él. Sus flows muestran su dirección y se pueden filtrar con device:. Ponle nombre desde uno de ellos, en Conexión.

# Looking for it…
t50ee7452 = Buscando…

# Installing. The system may ask you to confirm.
ta41c38ec = Instalando. Puede que el sistema te pida confirmación.

# Removing. The system may ask you to confirm.
t62b22726 = Quitando. Puede que el sistema pida confirmación.

# This computer trusts it.
t27d6bacc = Este equipo confía en él.

# Installed in: {places}.
tb5bcc244 = Instalado en: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Las herramientas de línea de comandos confían en él, pero los navegadores tienen sus propios almacenes. Hasta que esté en ellos, HTTPS a través de tracewing falla en un navegador.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing lee HTTPS con un certificado creado solo para este equipo. Hasta que se confíe en él, HTTPS a través de tracewing falla.

# Trust the certificate
t26618699 = Confiar en el certificado

# Install
tb50cbd40 = Instalar

# Also for command-line tools
t0db8faea = También para herramientas de línea de comandos

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = ¿Quitar el certificado de todos los almacenes de este ordenador? HTTPS a través de tracewing fallará hasta que se instale de nuevo. tracewing lo conserva para volver a instalarlo más tarde.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = En Linux, los navegadores necesitan certutil para que se les indique que confíen en él. Instálalo y luego pulsa Instalar aquí:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = En Linux, los navegadores necesitan certutil para que se les indique que confíen en él. Instala las herramientas NSS de tu distribución y luego pulsa Instalar aquí.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = Este escritorio no tiene un ajuste de proxy del sistema que tracewing pueda cambiar. Abre un navegador aparte desde Navegadores, o una terminal, más abajo.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } no tiene un ajuste de proxy del sistema que tracewing pueda cambiar. Abre un navegador aparte desde Navegadores, o una terminal, más abajo.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Las apps que usan el ajuste del sistema ahora pasan por { $addr }. El ajuste vuelve a como estaba cuando se cierra tracewing, o si falla.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Envía el tráfico de este equipo a través de tracewing, para las apps que usan el ajuste del sistema. O apunta tú mismo una app a { $addr }.

# Turn on the system proxy
t52b88441 = Activar el proxy del sistema

# Turn off
tcd03e04b = Desactivar

# Turn on
t65a0c5a7 = Activar

# Check it in your browser
tbed6e83a = Compruébalo en tu navegador

# Open the test page
te4e36f65 = Abrir la página de prueba

# Set up
t43c06abc = Configurar

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Tres pasos para ver el tráfico HTTPS de este equipo.

# See traffic
t980eee79 = Ver el tráfico

# the browsers' own stores
tddf6836c = los almacenes propios de los navegadores

# Trusts the certificate.
tc5bba858 = Confía en el certificado.

# Does not trust the certificate yet.
tf2a0b8c5 = Aún no confía en el certificado.

# Looking…
t284579ba = Buscando…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Aún no tiene perfil. Ábrelo una vez y vuelve a buscar.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 =
    { $n ->
        [one] { $holding } de su { $n } perfil confía en el certificado.
        [many] { $holding } de sus { $n } perfiles confían en el certificado.
       *[other] { $holding } de sus { $n } perfiles confían en el certificado.
    }

# Has not opened the test page yet.
t67f9cb49 = Aún no ha abierto la página de prueba.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Abrió la página de prueba a través de tracewing, por HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Abrió la página de prueba a través de tracewing, pero sin HTTPS.

# Browsers
td580efd4 = Navegadores

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Cada uno decide por su cuenta en qué confiar y adónde enviar su tráfico. Comprobar abre en él la página de prueba. Abrir aparte lo inicia con un perfil propio que pasa por tracewing, sin tocar el proxy del sistema.

# Look again
t98416222 = Volver a buscar

# No browser found on this computer.
t0e46bb34 = No se encontró ningún navegador en este ordenador.

# Check
t592c8de7 = Comprobar

# Open on its own
t0da7783c = Abrir aparte

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = El puerto UDP 443 está bloqueado en este equipo, así que las apps pasan de HTTP/3 a HTTP/2 y van por tracewing. El bloqueo termina cuando se cierra tracewing, o si falla.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Algunas apps usan HTTP/3, sobre UDP, y evitan el proxy. Bloquear el puerto UDP 443 en este equipo hace que vuelvan a HTTP/2 a través de tracewing. Requiere la contraseña de un administrador, y termina cuando se cierra tracewing. Mientras tanto, una VPN que use el puerto UDP 443 deja de funcionar.

# Apps on HTTP/3
t2cfe5270 = Apps con HTTP/3

# Stop blocking
t318358d8 = Dejar de bloquear

# Block HTTP/3
t88231e40 = Bloquear HTTP/3

# Or one app at a time
t91c63f1c = O una app cada vez

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Una terminal que pasa por tracewing mientras el proxy del sistema y las demás apps quedan como están. Para un navegador, Abrir aparte en Navegadores hace lo mismo con un perfil propio.

# Open a terminal
t4b629c72 = Abrir un terminal

# iPhone and iPad
ta550cc97 = iPhone y iPad

# Flutter apps
ta04ee397 = Apps de Flutter

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. En Ajustes, Wi-Fi, elige la red y luego Configurar proxy, Manual: servidor { $server }, puerto { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Escanea el código con la cámara, o abre { $page } en Safari, y descarga el certificado. Permite la descarga.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. En Ajustes, General, VPN y gestión de dispositivos, elige el perfil tracewing CA e instálalo.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. En Ajustes, General, Información, Ajustes de confianza de certificados, activa la confianza total para tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Abre https://{ $host } en Safari. Si dice que funciona, las apps que usan los ajustes del sistema pasan por tracewing.

# When you are done
t62466f08 = Cuando termines

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. En Ajustes, Wi-Fi, elige la red y luego Configurar proxy: Desactivado. Si queda activado, el dispositivo se queda sin Internet en esta red cuando tracewing se cierra.

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. En Ajustes, General, VPN y gestión de dispositivos, elige tracewing CA y elimina el perfil. Su confianza total se va con él.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. En Ajustes, Wi-Fi, edita la red y, en Opciones avanzadas, configura el proxy como Manual: host { $host }, puerto { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Escanea el código, o abre { $page } en el navegador, y descarga el certificado.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. En Ajustes, Seguridad, Cifrado y credenciales (los nombres varían según el fabricante), elige Instalar un certificado, luego Certificado de CA, y selecciona tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Desde Android 7, las apps solo confían en los certificados añadidos por el usuario si lo indican. Para una compilación de depuración de tu propia app, añade res/xml/network_security_config.xml:

# and point to it from AndroidManifest.xml:
t02ad844b = y haz referencia a él desde AndroidManifest.xml:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Las demás apps siguen rechazando el certificado: su tráfico aparece como flows fallidos.

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. En Ajustes, Wi-Fi, edita la red y vuelve a poner el proxy en Ninguno. Si queda activado, el dispositivo se queda sin Internet en esta red cuando tracewing se cierra.

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. En Ajustes, Seguridad, Encriptación y credenciales, Credenciales de usuario, elige tracewing y quítalo. Borra también tracewing.crt de Descargas.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = El HttpClient de Dart, que usan http y Dio en Flutter, no sigue el ajuste de proxy del teléfono, y en Android no confía en los certificados añadidos por el usuario. En una compilación de depuración, envíalo aquí:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback deja pasar cualquier certificado, por eso es solo para compilaciones de depuración.

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = Quita TracewingProxy y su línea en main() antes de publicar, aunque no hacen nada fuera de las compilaciones de depuración. En el teléfono no cambió nada.

# Open settings
t1336b4d6 = Abrir ajustes

# Phones and other devices
t2e3c591c = Teléfonos y otros dispositivos

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Un teléfono, una tableta u otro equipo de esta red puede pasar por tracewing en { $proxy }. El código abre la página con su certificado. Sus flows muestran su dirección, se pueden filtrar con device: y desde uno de ellos se le puede poner nombre al dispositivo, en Conexión.

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = Otros dispositivos no pueden llegar a tracewing en una de las direcciones de este equipo: { $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f =
    { $n ->
        [one] { $n } dispositivo pasó por tracewing ({ $names }). Cuando termines, desactiva su proxy Wi-Fi, como se explica abajo: si queda activado, se queda sin Internet en esta red cuando tracewing se cierra.
        [many] { $n } dispositivos pasaron por tracewing ({ $names }). Cuando termines, desactiva su proxy Wi-Fi, como se explica abajo: si queda activado, se quedan sin Internet en esta red cuando tracewing se cierra.
       *[other] { $n } dispositivos pasaron por tracewing ({ $names }). Cuando termines, desactiva su proxy Wi-Fi, como se explica abajo: si queda activado, se quedan sin Internet en esta red cuando tracewing se cierra.
    }

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Configurado. El certificado está entre los del sistema hasta que el dispositivo se reinicie.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Configurado. El certificado está en Descargas: en los Ajustes que se acaban de abrir, elige Cifrado y credenciales, Instalar un certificado, Certificado de CA, y selecciona tracewing.crt.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Las apps que solo confían en los certificados del sistema lo seguirán rechazando.

# An emulator.
t14b6b86d = Un emulador.

# On USB.
t67dafc14 = Por USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Acepta el aviso de depuración USB en el dispositivo y vuelve a buscar.

# Still starting. Look again in a moment.
t13f68b64 = Aún se está iniciando. Vuelve a buscar en un momento.

# adb says it is {state}.
t2b7d71da = adb indica el estado { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Configura el proxy en un emulador de Android, o en un teléfono conectado por USB con la depuración USB activada, e instala el certificado. El tráfico llega por ADB, así que ninguno tiene que estar en esta red.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = No se encontró adb. Instala Android Studio o las herramientas de plataforma de Android y vuelve a buscar.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = No hay ningún dispositivo conectado. Inicia un emulador, o conecta un teléfono con la depuración USB activada, y vuelve a buscar.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Configura un dispositivo para que envíe su tráfico aquí. Deshacer, o cerrar tracewing, le vuelve a quitar el proxy.

# Android emulators and USB devices
t379e433c = Emuladores de Android y dispositivos USB

# Find devices
t54d4af39 = Buscar dispositivos

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = El proxy del sistema y el bloqueo de HTTP/3 vuelven a como estaban cuando tracewing se cierra o falla, igual que un dispositivo Android configurado por USB. El certificado sigue siendo de confianza hasta que Quitar, en el paso 1, lo saque. Un teléfono configurado a mano conserva su proxy Wi-Fi y su certificado hasta que se quiten allí: la guía de cada dispositivo dice cómo. Otros dispositivos pueden usar el proxy de este ordenador hasta que se desactive en Ajustes.

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code y otras herramientas de IA pueden listar y leer los flows que graba tracewing, a través de su servidor MCP. Los tokens y las cookies quedan fuera salvo que los Ajustes los permitan. En Claude Code:

# Elsewhere, in the tool's MCP settings:
tec38833c = En otros sitios, en los ajustes de MCP de la herramienta:

# Flows
t4db77db2 = Flows

# Choose the flow above or below
tecb3adec = Elegir el flow de arriba o de abajo

# The first or the last flow
t7e278a00 = El primer o el último flow

# A page up or down
t23b1814b = Una página arriba o abajo

# Filter
tf4a9c097 = Filtrar

# Pause or resume recording
t6e21519a = Pausar o reanudar la grabación

# Clear the list
tf2a55d1f = Vaciar la lista

# Compare the flow clicked with the chosen one
te42cd067 = Comparar el flujo pulsado con el elegido

# The chosen flow
te0a729de = El flow elegido

# Its request
tb3970784 = Su solicitud

# Its response
t221fc168 = Su respuesta

# Send it again
tef7d1944 = Volver a enviarlo

# Edit it and send it
t6bc99e5c = Editarlo y enviarlo

# Copy it as cURL
tc99a11a3 = Copiarlo como cURL

# The part before or after the one shown
tdf4221d9 = La parte anterior o siguiente a la mostrada

# Save the body shown
t20d9b7c0 = Guardar el cuerpo mostrado

# Select all of the part shown
tf9168168 = Seleccionar todo lo de la parte mostrada

# Copy the selected text
t00b090d3 = Copiar el texto seleccionado

# Send it on, as edited
t8c35fa5a = Enviarlo tal como se editó

# Send it on as it was
t16f7abb5 = Enviarlo tal como estaba

# Drop it
t4e7f48b9 = Descartarlo

# The window
t2af18488 = La ventana

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Configurar, Tráfico, Reglas, En pausa, Componer, Sesiones, Ajustes

# Show or hide the sidebar
t67adca04 = Mostrar u ocultar la barra lateral

# Show or hide the inspector
ta5a7b9e9 = Mostrar u ocultar el inspector

# The inspector beside the list or below it
t51e214f7 = El inspector junto a la lista o debajo

# The next or the previous field
ta4b4bd5e = El campo siguiente o el anterior

# These shortcuts
tbdf3b62b = Estos atajos

# Close them
tf3a4b1ef = Cerrarlos

# Keyboard shortcuts
t31eaddb7 = Atajos de teclado

# Close
tcd86acc3 = Cerrar

# The signature in this body now works out to {now}, not {was}: it is no longer the transaction that was signed. tracewing holds no keys, so it cannot sign the new one.
td0d9c66b = La firma de este cuerpo ahora da { $now }, no { $was }: ya no es la transacción que se firmó. tracewing no guarda claves, así que no puede firmar la nueva.

# What this body carries no longer reads as a signed transaction. tracewing holds no keys, so it cannot sign it again.
td97ae94b = Lo que lleva este cuerpo ya no se lee como una transacción firmada. tracewing no guarda claves, así que no puede volver a firmarla.

# This body carries a signed transaction and its bytes have changed. The signatures are over the bytes as they were, and tracewing can neither check them nor make new ones.
ta6e69f9a = Este cuerpo lleva una transacción firmada y sus bytes han cambiado. Las firmas son sobre los bytes tal como estaban, y tracewing no puede comprobarlas ni crear otras.

# The certificate: {e}
t0235648c = El certificado: { $e }

# The terminal
tf542b2f4 = El terminal

# The system proxy: {e}
tc99726ce = El proxy del sistema: { $e }

# Blocking HTTP/3: {e}
t024ec871 = Bloqueo de HTTP/3: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx o texto en los cuerpos

# A name, as iPhone test
t19f89214 = Un nombre, como iPhone de pruebas

# A name, as Phone
tfce439bf = Un nombre, p. ej. Phone

# A message to send; {keys} sends it
tf9d9c09b = Un mensaje para enviar; { $keys } lo envía

# The search index: {e}
te3c971c9 = El índice de búsqueda: { $e }

# Click the flow to compare #{id} with.
tac1ec1d2 = Haz clic en el flujo que quieres comparar con #{ $id }.

# An earlier session: {name}
t73446a1d = Una sesión anterior: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Solo lectura. La búsqueda abarca más a medida que se completa su índice. La grabación continúa.

# Read-only. Recording goes on.
te2f35d9e = Solo lectura. La grabación continúa.

# Back to live
t317c0453 = Volver al directo

# {shown} of {total}
tf2cc0563 = { $shown } de { $total }

# Recording
te75cf20c = Grabando

# System proxy off · {addr}
t7e813d25 = Proxy del sistema desactivado · { $addr }

# System proxy…
t29e7dcd0 = Proxy del sistema…

# System proxy on · {addr}
t319ad6b9 = Proxy del sistema activado · { $addr }

# Apps
t53d91aad = Apps

# Devices
t27acf320 = Dispositivos

# All hosts
tfc7bfe37 = Todos los hosts

# No host has this in its name
t838c1d44 = Ningún host lo tiene en su nombre

# Hidden
t3c2dfe59 = Ocultos

# Hosts
tcf9aefb4 = Hosts

# Newest
t324b6d0b = Más recientes

# No flow fits this filter
t7eb295f4 = Ningún flow coincide con este filtro

# Nothing yet
tbd00747c = Nada todavía

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Activa el proxy del sistema arriba, o apunta una app a { $addr }. Para probarlo: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Junto a la lista

# Below the list
t23658daf = Debajo de la lista

# Drop it on a place · Esc puts it back
t0fa632ba = Suéltalo en un sitio · Esc lo devuelve a su lugar

# Traffic
t4b589266 = Tráfico

# Certificate trusted on this computer
t7b93f551 = Certificado de confianza en este equipo

# Checking the certificate…
tbf8942c3 = Comprobando el certificado…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Instalando el certificado; puede que el sistema te pida confirmación…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Quitando el certificado; puede que el sistema pida confirmación…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS fallará hasta que se confíe en el certificado: configurar

# settings.toml was not used: {problem}
tc3e1f9c8 = No se usó settings.toml: { $problem }

# A setting was not saved: {problem}
tff1a5018 = Un ajuste no se guardó: { $problem }

# The proxy stopped: {failure}
t04dcff11 = El proxy se detuvo: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = El disco está casi lleno: no se están guardando los cuerpos

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
        [one] { $count } flow no grabado
       *[other] { $count } flows no grabados
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
        [one] { $count } flow sin cuerpos
       *[other] { $count } flows sin cuerpos
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
        [one] { $count } flow más antiguo descartado para no superar { $size }
       *[other] { $count } flows más antiguos descartados para no superar { $size }
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
        [one] { $count } flow
       *[other] { $count } flows
    }

# Export as HAR
t95b6b766 = Exportar como HAR

# Shortcuts {keys}
t4ab0ecb4 = Atajos { $keys }

# Columns
tea156fdc = Columnas

# base64url
tadfc8ac3 = base64url

# base64
tf04e6eba = base64

# Hex
t613f34ea = Hex

# Percent-encoded
t7c7bb5a9 = Codificado en porcentaje

# Escaped text
t195b3db9 = Texto con escapes

# HTML entities
t54958ecb = Entidades HTML

# Version
t5dcdd537 = Versión

# Variant
t8433b026 = Variante

# UUID
t257b8832 = UUID

# ULID
tf19e9db9 = ULID

# ObjectId
t427abd77 = ObjectId

# Counter
tfc0f33c3 = Contador

# KSUID
te30e0b47 = KSUID

# Worker
tde5c1a77 = Worker

# Sequence
t64cec188 = Secuencia

# Snowflake
te00a2b87 = Snowflake

# JSON, {n} fields
tc6a0fbd2 = JSON, { $n } campos

# JSON, {n} items
t06f5832d = JSON, { $n } elementos

# JSON in a string
tc69f7572 = JSON dentro de una cadena

# Expires
tc7aa0523 = Caduca

# Issued
tb1835ecc = Emitido

# Not before
td10dacd9 = No antes de

# Algorithm
t9945210a = Algoritmo

# State
t1deca496 = Estado

# expired
t338eb632 = caducado

# JWT
t6076261e = JWT

# {n} row | {n} rows
td8d653d6 =
    { $n ->
        [one] { $n } fila
        [many] { $n } filas
       *[other] { $n } filas
    }

# In hex and binary
tc31ef85f = En hex y binario

# Bits set
tb67e8e09 = Bits activos

# As a size
ta9175b27 = Como tamaño

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
tfda57c3e = en { $amount }

# {amount} ago
t5f968014 = hace { $amount }

# Unix seconds
t6edf5796 = Segundos Unix

# Unix milliseconds
t0d28f4b7 = Milisegundos Unix

# Unix microseconds
tc705fe24 = Microsegundos Unix

# Unix nanoseconds
t235cb430 = Nanosegundos Unix

# ISO 8601
t402bdff3 = ISO 8601

# ISO 8601, no zone: read as UTC
t3bb6c7ca = ISO 8601, sin zona: leído como UTC

# HTTP-date
te55df34c = HTTP-date

# Seconds
t7e12f48a = Segundos

# Trace context
ta58c7c95 = Contexto de traza

# Trace
t26f8e74e = Traza

# Span
taba837e1 = Span

# Sampled
t7e4a1d99 = Muestreado

# yes
t4e9f3590 = sí

# no
t62366ffa = no

# Parent span
t3cf3f9c3 = Span padre

# B3 trace
tb4f9b9bb = Traza B3

# Began
t3a1b8566 = Comenzó

# Amazon trace
t122e82a4 = Traza de Amazon

# Server-Timing
tb79773df = Server-Timing

# Took
tcea1562e = Duración

# About
t58e096de = Descripción

# Scheme
t9c9083a2 = Esquema

# Port
te2c53cc6 = Puerto

# User
te0a63e12 = Usuario

# {n} fields
t98976d50 = { $n } campos

# Fragment
t7889e7fd = Fragmento

# Query fields
t2f58f1e0 = Campos de la consulta

# Field
t0fb83d07 = Campo

# Value
td147f96a = Valor

# Basic authentication
te161b66b = Autenticación Basic

# Password
t2cc30838 = Contraseña

# never kept
t83c57deb = nunca se guarda

# kept {span}
t01f67b47 = se guarda { $span }

# kept {span} by shared caches
t5af054c4 = se guarda { $span } en cachés compartidas

# by the browser only
tfb7516b7 = solo por el navegador

# by anyone
t86fe63fe = por cualquiera

# checked every time
t02076cba = se comprueba cada vez

# not used once stale
t0f7f5bf9 = no se usa si está obsoleto

# never checked again
tc01144d0 = nunca se vuelve a comprobar

# Cache-Control
t3868d34d = Cache-Control

# Status code
t20b704bc = Código de estado

# Cookie
tbdbabadf = Cookie

# data: URI, {media}
t514d8ddb = URI data:, { $media }
