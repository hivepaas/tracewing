# tracewing in Spanish. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {size} more
ta8dc6eab = … { $size } más

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } línea
       *[other] { $size } · { $count } líneas
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · leyendo { $percent }%

# Appearance: {now}. Click for {next}.
t332cfa6d = Apariencia: { $now }. Haz clic para pasar a { $next }.

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

# Size
ta6478e7c = Tamaño

# Time
tdfe4e404 = Tiempo

# Number
t61e9eac0 = Número

# "{method}" is not a method
tacdcf5cb = "{ $method }" no es un método

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = el cuerpo de su solicitud no se guardó completo, así que no se puede volver a enviar

# name: value, one a line
t4df37bea = nombre: valor, uno por línea

# No body
tfa4c2902 = Sin cuerpo

# No flow is chosen
t550b0a98 = Ningún flow seleccionado

# This flow: {e}
t296762f8 = Este flow: { $e }

# Request
t82454152 = Solicitud

# Headers
t334f5989 = Encabezados

# Body
t428a1095 = Cuerpo

# Sending…
te946e7bf = Enviando…

# Send
tf28e14cf = Enviar

# New
t8b242db1 = Nueva

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … { $count } bytes más: el flow en Tráfico los tiene todos.

# Response
t55a982fe = Respuesta

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

# Not decrypted
t8090a1ba = No descifrado

# open
td35ec4c9 = abierto

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

# Expires
tc7aa0523 = Caduca

# Issued
tb1835ecc = Emitido

# Not before
td10dacd9 = No antes de

# in {amount}
tfda57c3e = en { $amount }

# {amount} ago
t5f968014 = hace { $amount }

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

# URL
t7569633e = URL

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

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = No hay nada en pausa. Añade una regla de pausa en la pantalla Reglas para detener el tráfico aquí.

# for {method} {url}
t093a86f6 = para { $method } { $url }

# The request, before the server gets it
tc840fed6 = La solicitud, antes de que la reciba el servidor

# The response, before the app gets it
t6683d72a = La respuesta, antes de que la reciba la app

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

# Add
t9dc3aa14 = Añadir

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

# Exported HAR files leave out tokens and cookies
td4a2f543 = Los archivos HAR exportados omiten tokens y cookies

# Other devices
t9e7d8362 = Otros dispositivos

# Let other devices use the proxy
t13e626b3 = Permitir que otros dispositivos usen el proxy

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust. Takes effect when tracewing starts again.
t2e14229d = Así, los teléfonos y otros equipos de tu red pueden enviar su tráfico aquí. También cualquier otra persona en ella: déjalo desactivado en una red en la que no confíes. Se aplica al reiniciar tracewing.

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

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Mantener las apps fuera de HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them. Takes effect when tracewing starts again.
t2aa7be0d = HTTP/3 va sobre UDP, que ningún proxy transporta, así que una app que lo usa evita tracewing. Esto quita de las respuestas de los servidores sus ofertas de HTTP/3, y las apps se quedan en HTTP/2. Los flows siguen mostrando las ofertas tal como las enviaron los servidores. Se aplica al reiniciar tracewing.

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

# Saved. The limits apply the next time tracewing starts.
tc8bd2065 = Guardado. Los límites se aplican la próxima vez que se inicie tracewing.

# Settings
t4b058728 = Ajustes

# Back to defaults
ta3162a13 = Restablecer valores predeterminados

# Hosts not to decrypt
tb457f0b3 = Hosts que no descifrar

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies as soon as it is saved.
t7647c310 = Uno por línea: un host, o *.example.com para él y todo lo que hay debajo. Su tráfico pasa sin leerse. Esta lista se aplica en cuanto se guarda.

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

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings, then start tracewing again.
t9f95b7b8 = Un teléfono u otro equipo también puede enviar su tráfico aquí. Activa “Permitir que otros dispositivos usen el proxy” en Ajustes y luego reinicia tracewing.

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

# Remove
t21a5901d = Quitar

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

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = El HttpClient de Dart, que usan http y Dio en Flutter, no sigue el ajuste de proxy del teléfono, y en Android no confía en los certificados añadidos por el usuario. En una compilación de depuración, envíalo aquí:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback deja pasar cualquier certificado, por eso es solo para compilaciones de depuración.

# Open settings
t1336b4d6 = Abrir ajustes

# Phones and other devices
t2e3c591c = Teléfonos y otros dispositivos

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Un teléfono, una tableta u otro equipo de esta red puede pasar por tracewing en { $proxy }. El código abre la página con su certificado. Sus flows muestran su dirección, se pueden filtrar con device: y desde uno de ellos se le puede poner nombre al dispositivo, en Conexión.

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

# Undo
t71fd4acf = Deshacer

# Android emulators and USB devices
t379e433c = Emuladores de Android y dispositivos USB

# Find devices
t54d4af39 = Buscar dispositivos

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

# Select all of the part shown
tf9168168 = Seleccionar todo lo de la parte mostrada

# Copy the selected text
t00b090d3 = Copiar el texto seleccionado

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

# These shortcuts
tbdf3b62b = Estos atajos

# Close them
tf3a4b1ef = Cerrarlos

# Keyboard shortcuts
t31eaddb7 = Atajos de teclado

# Close
tcd86acc3 = Cerrar

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

# The search index: {e}
te3c971c9 = El índice de búsqueda: { $e }

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

# Hosts
tcf9aefb4 = Hosts

# All hosts
tfc7bfe37 = Todos los hosts

# Hidden
t3c2dfe59 = Ocultos

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
