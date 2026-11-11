# tracewing in Brazilian Portuguese. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 =
    { $n ->
        [one] … { $count } linha { $closer }
        [many] … { $count } linhas { $closer }
       *[other] … { $count } linhas { $closer }
    }

# … {size} more
ta8dc6eab = … mais { $size }

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc =
    { $n ->
        [one] { $size } { $format } · { $count } linha de JSON
        [many] { $size } { $format } · { $count } linhas de JSON
       *[other] { $size } { $format } · { $count } linhas de JSON
    }

# Unfold all
t3849d44e = Expandir tudo

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } linha
       *[other] { $size } · { $count } linhas
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · lendo { $percent }%

# Appearance: {now}. Click for {next}.
t332cfa6d = Aparência: { $now }. Clique para { $next }.

# Method
t98a2c750 = Método

# Status
t005ef20f = Status

# App
t81f51acc = App

# Host
t302af18f = Host

# Path
teb66e456 = Caminho

# Operation
t218e02a6 = Operação

# Size
ta6478e7c = Tamanho

# Time
tdfe4e404 = Tempo

# Number
t61e9eac0 = Número

# "{method}" is not a method
tacdcf5cb = "{ $method }" não é um método

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = o corpo da requisição não foi guardado inteiro, então ela não pode ser reenviada

# name: value, one a line
t4df37bea = nome: valor, um por linha

# No body
tfa4c2902 = Sem corpo

# No flow is chosen
t550b0a98 = Nenhum fluxo escolhido

# This flow: {e}
t296762f8 = Este fluxo: { $e }

# Request
t82454152 = Requisição

# Headers
t334f5989 = Cabeçalhos

# Body
t428a1095 = Corpo

# Sending…
te946e7bf = Enviando…

# Send
tf28e14cf = Enviar

# New
t8b242db1 = Nova

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … mais { $count } bytes: o fluxo em Tráfego tem todos.

# Response
t55a982fe = Resposta

# Compose
t18049f93 = Compor

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Uma requisição sua, ou uma reenviada. Ela passa pelo tracewing, então as regras se aplicam e ela também aparece em Tráfego.

# {version} both sides
t961a5ea0 = { $version } nos dois lados

# {client} to the proxy, {server} to the server
t3d237646 = { $client } até o proxy, { $server } até o servidor

# Queued
t9e437c74 = Na fila

# Connect
tdb89a899 = Conexão

# Waiting
tc9450e48 = Espera

# Download
t6964bb19 = Download

# No file picker: {e}
t6935fbd3 = Sem seletor de arquivos: { $e }

# Writing the HAR…
t725a0e89 = Gravando o HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
        [one] { $n } fluxo gravado em { $path }
       *[other] { $n } fluxos gravados em { $path }
    }

# The HAR was not written: {e}
tef8ec569 = O HAR não foi gravado: { $e }

# Open
t538b10e9 = Abrir

# Reading {path}…
tc0e7c8f4 = Lendo { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
        [one] , { $n } entrada deixada de fora
       *[other] , { $n } entradas deixadas de fora
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
        [one] { $n } fluxo lido de { $format }
       *[other] { $n } fluxos lidos de { $format }
    }

# The file was not read: {e}
t7f49510f = O arquivo não foi lido: { $e }

# it is no longer kept
ta7b4c380 = não está mais guardado

# {size} saved to {path}
tc6932467 = { $size } salvo em { $path }

# The body was not saved: {e}
t189900b4 = O corpo não foi salvo: { $e }

# Fiddler session
t9f5c89a3 = Sessão do Fiddler

# Charles session
tb4f56f2b = Sessão do Charles

# All
t75afe364 = Todos

# Errors
te9ed60a6 = Erros

# Media
td10fcaa3 = Mídia

# Rule: {rule}
teca2074f = Regra: { $rule }

# Events
t003420a4 = Eventos

# JSON lines
te8ce2db0 = Linhas JSON

# Not decrypted
t8090a1ba = Não descriptografado

# open
td35ec4c9 = aberto

# Reading the request…
t51c96267 = Lendo a requisição…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = O corpo da requisição não foi guardado, então sua query não pode ser mostrada.

# An operation without a name
t38c4d30b = Uma operação sem nome

# {n} of {count}: {name}
t53045460 = { $n } de { $count }: { $name }

# Query
t9e89e783 = Query

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = Uma persisted query: o servidor guarda o texto, e a requisição envia só o hash.

# Variables
t7996f062 = Variáveis

# Cut
t58e25863 = Recortar

# Copy
t658f3664 = Copiar

# Paste
tb34a0288 = Colar

# Select all
t6af21072 = Selecionar tudo

# This flow is gone.
t3343999e = Este fluxo não existe mais.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Este host não é descriptografado, então o que ele envia não pode ser visto.

# Still coming: it shows once it ends.
t42be95fb = Ainda chegando: aparece quando terminar.

# No body.
t31e4d044 = Sem corpo.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Este corpo não foi guardado: o disco estava quase cheio, ou havia coisa demais esperando por ele.

# This body was not kept.
te46616b5 = Este corpo não foi guardado.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Este corpo não existe mais: foi descartado com os fluxos mais antigos, ou não pode ser lido.

# Cannot open this body: {e}
t620a039b = Não é possível abrir este corpo: { $e }

# Inspector
t4def410e = Inspetor

# Choose a flow to see it here
tc7be91e8 = Escolha um fluxo para vê-lo aqui

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Não descriptografado. Novas conexões com este host passam sem ser lidas.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Se um app falhar com este host, talvez ele fixe os próprios certificados (pinning).

# Make a rule
t0ac0e062 = Criar uma regra

# Decrypt this host
tb7367e2c = Descriptografar este host

# Do not decrypt this host
t5fa113f6 = Não descriptografar este host

# Request and response
t04e060e6 = Requisição e resposta

# Request and response one at a time
t70397119 = Requisição e resposta, uma de cada vez

# Request and response side by side
t3d35ce69 = Requisição e resposta lado a lado

# Request above the response
t83c63123 = Requisição acima da resposta

# Pretty
t7e09470d = Formatado

# Raw
ta689aa91 = Bruto

# Picture · {size}
te97ed633 = Imagem · { $size }

# Messages {count}
te61b841a = Mensagens { $count }

# Headers {headers}
t04b0146d = Cabeçalhos { $headers }

# Cookies {cookies}
td58eb84b = Cookies { $cookies }

# Query {query}
t4756f9f5 = Query { $query }

# Form {form}
tfa6e7203 = Formulário { $form }

# Connection
t09ea4ff9 = Conexão

# Save body
te2c44424 = Salvar corpo

# Send again
tf5910557 = Reenviar

# Edit and send
tf837e542 = Editar e enviar

# Copied
t8dc21305 = Copiado

# Copy as cURL
t866d1e3e = Copiar como cURL

# Copy as…
t9c1ad448 = Copiar como…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Esta imagem não pode ser exibida. Salve o corpo para vê-la.

# Opening…
tc40f2321 = Abrindo…

# No response
t8c2c37b1 = Sem resposta

# Tunnel, not decrypted
tce693254 = Túnel, não descriptografado

# GraphQL: {operation}
t2d65757d = GraphQL: { $operation }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Pela conexão { $id }, feita antes: nenhum tempo foi gasto conectando.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Não encontrado: pode ter terminado, ou pertencer a outro usuário

# On another device
t2b964a37 = Em outro dispositivo

# CLIENT
t6d3cd99e = CLIENTE

# Device
tbdcbb413 = Dispositivo

# Program
tc4253b6b = Programa

# Address
t700560f3 = Endereço

# SERVER
te3b27612 = SERVIDOR

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = O que era esta conexão não está mais guardado: foi junto com fluxos mais antigos.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Não descriptografado: o tracewing o repassou sem olhar.

# No connection was made.
tf348eb92 = Nenhuma conexão foi feita.

# None: it was answered without one.
tac71808e = Nenhuma: foi respondido sem conexão.

# {id}, given up during TLS
t0ec66a8c = { $id }, abandonada durante o TLS

# {id}, made for this flow
t7fe32cfc = { $id }, feita para este fluxo

# {id}, made before and used again
ta904c078 = { $id }, feita antes e reutilizada

# Server
t6d7302f2 = Servidor

# None: plain HTTP
t05606349 = Nenhum: HTTP simples

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = O SERVIDOR NÃO MOSTROU CERTIFICADOS

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = O QUE O SERVIDOR MOSTROU, O DELE PRIMEIRO

# NO CERTIFICATES
te6f31dce = SEM CERTIFICADOS

# CERTIFICATE
t1c902790 = CERTIFICADO

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
        [one] { $n } CERTIFICADO, O DO SERVIDOR PRIMEIRO
       *[other] { $n } CERTIFICADOS, O DO SERVIDOR PRIMEIRO
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = O tracewing não continuou com este servidor: { $error }. Se o app confia nele de outra forma, como por uma CA da empresa, escolha “Não descriptografar este host” e o app verificará o servidor por conta própria.

# Cipher
t5b3bed6a = Cifra

# none
tada7afdb = nenhuma

# not valid until {when}
t0a14b2d8 = válido só a partir de { $when }

# expired {when}
t932538c2 = expirou { $when }

# expires {when}
t679bcb51 = expira { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
        [one] { $names } e mais { $n }
       *[other] { $names } e mais { $n }
    }

# Issued by {issuer}
t15a1ce5a = Emitido por { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Válido de { $from } a { $to } · { $when }

# Names {names}
tda75e459 = Nomes { $names }

# Serial {serial}
td1f0d98b = Série { $serial }

# These headers were not kept.
tb8739fec = Estes cabeçalhos não foram guardados.

# No headers.
tf3dacb5e = Sem cabeçalhos.

# Trailers, sent after the body
t3447e9de = Trailers, enviados depois do corpo

# File {filename} · {kind} · {size}
t6dbd239f = Arquivo { $filename } · { $kind } · { $size }

# no type
t902a6ece = sem tipo

# No cookies were sent.
t58e85ab2 = Nenhum cookie foi enviado.

# No cookies were set.
tf57dbabc = Nenhum cookie foi definido.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · assinatura não verificada

# JWT · signature not checked
t05f8b533 = JWT · assinatura não verificada

# Header
t114f9980 = Cabeçalho

# Payload
ta0639745 = Payload

# Expires
tc7aa0523 = Expira

# Issued
tb1835ecc = Emitido

# Not before
td10dacd9 = Não antes de

# in {amount}
tfda57c3e = em { $amount }

# {amount} ago
t5f968014 = há { $amount }

# Light
t8ad2f8cf = Claro

# Dark
t45570065 = Escuro

# System
t8f3184bc = Sistema

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
        [one] transmitido em { $count } parte
       *[other] transmitido em { $count } partes
    }

# stopped: {stop}
tfa48fb30 = parou: { $stop }

# {input} in · {output} out tokens
tc8bbc724 = tokens: { $input } de entrada · { $output } de saída

# Reading the call…
t33b52a3c = Lendo a chamada…

# Answer
tf94ea0ff = Resposta

# Thinking: {thinking}
t737ece33 = Raciocínio: { $thinking }

# Calls {name}{id}
t017882e4 = Chama { $name }{ $id }

# Nothing yet.
td9b71d16 = Nada ainda.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
        [one] { $text }… e mais { $count } caractere
       *[other] { $text }… e mais { $count } caracteres
    }

# Copy the answer
t69975ed7 = Copiar a resposta

# Asked
t4e3af9c3 = Pergunta

# Tools offered: {tools}
tb871bcf4 = Ferramentas oferecidas: { $tools }

# Not kept
tba30b5fa = Não guardado

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Esta mensagem não foi guardada: havia coisa demais esperando, ou o disco estava quase cheio.

# This message is empty.
tdb0ec01f = Esta mensagem está vazia.

# No messages yet
t51e7624c = Nenhuma mensagem ainda

# {said} kept, of {size}
t3558745a = { $said } guardados, de { $size }

# {what}: {text} is not a whole number
t2a8515ce = { $what }: { $text } não é um número inteiro

# Latency
t23a45ef9 = Latência

# Down
ta4995035 = Download

# Up
t42f35940 = Upload

# Loss
t69db303a = Perda

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Informe uma latência, uma velocidade ou uma perda para deixar a rede mais lenta

# Latency, ms
tefe18ab9 = Latência, ms

# Down, kbit/s
t08c4ae33 = Download, kbit/s

# Up, kbit/s
td55cfe9a = Upload, kbit/s

# Loss, %
tea4af439 = Perda, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Só estes hosts, como *.example.com; todos se vazio

# Not kept for next time: {e}
tdce4d67c = Não guardado para a próxima vez: { $e }

# Slow network
td51ddfce = Rede lenta

# Off: traffic goes as fast as it can
tf0f34db0 = Desligado: o tráfego vai o mais rápido que puder

# Custom
t9525b78e = Personalizado

# Only these hosts
tfe16992b = Só estes hosts

# Apply
tca77bbdb = Aplicar

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = Vale a partir da próxima requisição. A latência é somada a cada ida e volta; a velocidade é um link que todos os fluxos desacelerados compartilham; um pedaço perdido espera para ser reenviado, como no TCP. Os outros hosts não são desacelerados.

# Header line {line} has no ":"
t891b076d = A linha de cabeçalho { $line } não tem ":"

# Header line {line} has no name
te3247b96 = A linha de cabeçalho { $line } não tem nome

# URL
t7569633e = URL

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
        [one] { $n } byte que não é texto: segue como chegou.
       *[other] { $n } bytes que não são texto: seguem como chegaram.
    }

# Nothing is paused
t8cf68b4c = Nada está pausado

# "{status}" is not a status
t40a529af = "{ $status }" não é um status

# REQUEST
t1f3f6dd2 = REQUISIÇÃO

# RESPONSE
tc85f77de = RESPOSTA

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Nada está pausado. Adicione uma regra de Pausa na tela Regras para parar o tráfego aqui.

# for {method} {url}
t093a86f6 = para { $method } { $url }

# The request, before the server gets it
tc840fed6 = A requisição, antes de o servidor recebê-la

# The response, before the app gets it
t6683d72a = A resposta, antes de o app recebê-la

# Send as it came
tb593e730 = Enviar como chegou

# Drop
t108524ac = Descartar

# Paused
tde281feb = Pausados

# Nothing is waiting.
tea613769 = Nada está esperando.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
        [one] { $n } aguardando envio.
       *[other] { $n } aguardando envio.
    }

# Send all as they came
tdce25640 = Enviar todos como chegaram

# The list was not saved: {e}
tfceba380 = A lista não foi salva: { $e }

# Add
t9dc3aa14 = Adicionar

# Reading…
t6aad8485 = Lendo…

# {count} file | {count} files
tde674bd8 =
    { $n ->
        [one] { $count } arquivo
        [many] { $count } arquivos
       *[other] { $count } arquivos
    }

# {count} message | {count} messages
t8a1cd2f6 =
    { $n ->
        [one] { $count } mensagem
        [many] { $count } mensagens
       *[other] { $count } mensagens
    }

# {count} service | {count} services
t51f71902 =
    { $n ->
        [one] { $count } serviço
        [many] { $count } serviços
       *[other] { $count } serviços
    }

# and {count} more problem | and {count} more problems
t591ce5ea =
    { $n ->
        [one] e mais { $count } problema
        [many] e mais { $count } problemas
       *[other] e mais { $count } problemas
    }

# Remove
t21a5901d = Remover

# Protobuf schemas
ta4401d1a = Esquemas Protobuf

# Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
te5f4f07c = Pastas de arquivos .proto, ou descriptor sets feitos com protoc -o, para mostrar corpos Protobuf e gRPC com os nomes dos campos. Uma chamada gRPC encontra suas mensagens pelo caminho; os demais Protobuf, pelo messageType que o tipo indica. São lidos de novo quando mudam.

# Add a folder…
tb5930e43 = Adicionar uma pasta…

# Add files…
ted08dbe5 = Adicionar arquivos…

# Read again
t2f0dcc5d = Ler de novo

# Find a message
te6f78626 = Buscar uma mensagem

# Read as {message}
t87b6a380 = Ler como { $message }

# Read as…
t1b90af95 = Ler como…

# Requests to {path} on {host} are read as the message chosen.
t8826d766 = Requisições para { $path } em { $host } são lidas como a mensagem escolhida.

# Answers from {path} on {host} are read as the message chosen.
tbbe012da = Respostas de { $path } em { $host } são lidas como a mensagem escolhida.

# Automatic (gRPC path or messageType)
t134c9a06 = Automático (caminho gRPC ou messageType)

# and {count} more: type to narrow them | and {count} more: type to narrow them
t70ff68f9 =
    { $n ->
        [one] e mais { $count }: digite para filtrar
        [many] e mais { $count }: digite para filtrar
       *[other] e mais { $count }: digite para filtrar
    }

# Block
tbd074002 = Bloquear

# Answer from a file
td8edab50 = Responder com um arquivo

# Send to another host
te3cd85d7 = Enviar para outro host

# Set a request header
tfad4aaaa = Definir um cabeçalho de requisição

# Set a response header
tcbbc20d0 = Definir um cabeçalho de resposta

# Remove a request header
t5228dd84 = Remover um cabeçalho de requisição

# Remove a response header
teeb5735a = Remover um cabeçalho de resposta

# Set a query parameter
t73d9419b = Definir um parâmetro de query

# Remove a query parameter
t12633da9 = Remover um parâmetro de query

# Change the request body
t41101201 = Mudar o corpo da requisição

# Change the response body
tdd259b73 = Mudar o corpo da resposta

# Change the status
t4bbdfba8 = Mudar o status

# Pause the request
t40ecc39b = Pausar a requisição

# Pause the response
t18a34799 = Pausar a resposta

# Status to answer with, like 403
t8f3ebb6c = Status da resposta, como 403

# Full path of a file or folder
t9fb9ae4c = Caminho completo de um arquivo ou pasta

# Where to, like http://localhost:3000
tdfedd004 = Para onde, como http://localhost:3000

# Header's name
t80793085 = Nome do cabeçalho

# Its value
t43968778 = Valor

# Parameter's name
t313c8e35 = Nome do parâmetro

# Text to find, or empty for the whole body
t8255fc87 = Texto a procurar, ou vazio para o corpo inteiro

# What to put in its place
t03671fd5 = O que vai no lugar

# Status the client gets, like 500
t5e4d3088 = Status que o cliente recebe, como 500

# "{first}" is not a status
td43128d3 = "{ $first }" não é um status

# Block with {status}
t1727e984 = Bloquear com { $status }

# Answer from {path}
taefc08fa = Responder com { $path }

# Send to {to}
t5e256873 = Enviar para { $to }

# Request: {header}: {value}
taa230766 = Requisição: { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Resposta: { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Requisição: sem { $header }

# Response: no {header}
t23960a2e = Resposta: sem { $header }

# Query: {name}={value}
t0ee6c1c0 = Query: { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Query: sem { $name }

# Status {status}
tad048e53 = Status { $status }

# Request body: {replace}
te4856c36 = Corpo da requisição: { $replace }

# Response body: {replace}
t8600dc42 = Corpo da resposta: { $replace }

# Request body: {find} → {replace}
taec471a9 = Corpo da requisição: { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Corpo da resposta: { $find } → { $replace }

# Name
t0fe07306 = Nome

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, como api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Método, ou vazio para todos

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
        [one] { $n } regra gravada em { $path }
       *[other] { $n } regras gravadas em { $path }
    }

# The rules were not written: {e}
te7ac257f = As regras não foram gravadas: { $e }

# Import
t573e0af4 = Importar

# {path} could not be read: {e}
t32a3ce94 = Não foi possível ler { $path }: { $e }

# Nothing was imported: {e}
tb9f7257a = Nada foi importado: { $e }

# Edit
tc2c76cb1 = Editar

# Delete
t5797ea6a = Excluir

# Change this rule
t9160f393 = Alterar esta regra

# A new rule
t55a722a0 = Uma nova regra

# It waits under Paused until you send it on, changed or not.
tca566a57 = Ela espera em Pausados até você enviá-la, alterada ou não.

# Save
t4d2d5d68 = Salvar

# Cancel
t35afca3b = Cancelar

# Rules
t428b2d80 = Regras

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Valem a partir da próxima requisição, também com a gravação em pausa. De cima para baixo: o primeiro bloqueio, arquivo ou host vence, e toda outra regra que se encaixa se aplica, em ordem. Uma regra que muda um corpo ou um status muda o que o servidor enviou.

# Import rules…
t5edc205d = Importar regras…

# Export rules…
t6e09ca20 = Exportar regras…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
        [one] { $n } regra adicionada
       *[other] { $n } regras adicionadas
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
        [one] { $n } já existia
       *[other] { $n } já existiam
    }

# left out: {why}
ta81e95e2 = deixado de fora: { $why }

# Copy all
t6aef5c37 = Copiar tudo

# Too large to copy all
t2fe8ff9f = Grande demais para copiar tudo

# No flows
t6dc463eb = Nenhum fluxo

# Name it to keep it
t52c1a402 = Dê um nome para mantê-la

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · gravada por outra versão, então não pode ser aberta

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
        [one] { $count } fluxo · { $size }
       *[other] { $count } fluxos · { $size }
    }

# Recording now
t3cb3e6fc = Gravando agora

# Export HAR
t4d6458cc = Exportar HAR

# Delete for good?
ta7737f13 = Excluir de vez?

# Sessions
tb69a8aec = Sessões

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Uma sessão com nome é mantida até você excluí-la. Das outras, ficam as { $kept } mais recentes.

# Working…
td5507854 = Trabalhando…

# Open a session file
tae8622fc = Abrir um arquivo de sessão

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, .saz do Fiddler ou .chlsj do Charles

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = Os arquivos HAR exportados deixam de fora cookies, tokens, chaves e senhas, em cabeçalhos, URLs e corpos

# Other devices
t9e7d8362 = Outros dispositivos

# Let other devices use the proxy
t13e626b3 = Permitir que outros dispositivos usem o proxy

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = Celulares e outros computadores na sua rede poderão então enviar o tráfego deles para cá. Qualquer outra pessoa nela também: deixe isto desligado em uma rede em que você não confia.

# Servers
tcd0cf113 = Servidores

# Trust what this computer trusts
tfea2d91b = Confiar no que este computador confia

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = Servidores com um certificado de uma autoridade instalada neste computador, como a da própria empresa, são acessados como os navegadores deste computador os acessam. Desligado, só as autoridades públicas são confiáveis. Vale quando o tracewing for reiniciado.

# AI tools
t294d7ea4 = Ferramentas de IA

# Let AI tools see tokens and cookies
te1d26d3c = Permitir que ferramentas de IA vejam tokens e cookies

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = Ferramentas de IA leem o tráfego gravado pelo servidor MCP do tracewing. Desligado, os valores de Authorization, Cookie e Set-Cookie ficam de fora do que elas recebem. Vale na hora.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Manter os apps fora do HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = O HTTP/3 usa UDP, que nenhum proxy transporta, então um app que o usa contorna o tracewing. Isto tira das respostas as ofertas de HTTP/3 dos servidores, e os apps ficam no HTTP/2. Os fluxos ainda mostram as ofertas como os servidores as enviaram.

# Size of a session
ta20fa102 = Tamanho de uma sessão

# Past this, the oldest flows are dropped.
td2914f46 = Acima disto, os fluxos mais antigos são descartados.

# Room to leave on the disk
t8674a3b3 = Espaço a deixar livre no disco

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Com menos que isto livre, os fluxos ainda são listados, mas os corpos não são guardados.

# Earlier sessions to keep
td356194a = Sessões anteriores a manter

# Older ones are deleted when a new session starts.
t66556513 = As mais antigas são excluídas quando uma nova sessão começa.

# Bodies
tdb4d0409 = Corpos

# Most kept of one body
t777508dd = Máximo guardado de um corpo

# It still reaches the client whole.
t386fee28 = Ele ainda chega inteiro ao cliente.

# Most unpacked for viewing
tc3fe2efd = Máximo descompactado para exibição

# Stops a small download that unpacks to many GB.
tf9c49515 = Impede que um download pequeno se descompacte em muitos GB.

# Small body size
ta8996af1 = Tamanho de corpo pequeno

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Corpos menores esperam na memória e dividem um arquivo. Os maiores ganham um arquivo próprio conforme chegam.

# Compress plain text on disk
tee503c2e = Compactar texto simples no disco

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Ocupa cerca de metade do espaço, e é pulado quando o tráfego é rápido demais para isso.

# Compression level
td386c9ef = Nível de compactação

# 1 to 19. Higher is smaller and slower.
td42a987d = 1 a 19. Mais alto é menor e mais lento.

# Search
tc646a2c9 = Pesquisa

# Search URLs
t0c3e0b3d = Pesquisar URLs

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Encontra qualquer parte de uma URL enquanto você digita. Custa pouco.

# Search bodies
t8eec0293 = Pesquisar corpos

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Encontra palavras nos corpos. Pode ocupar tanto disco quanto os próprios corpos.

# Searched part of each body
tab293b21 = Parte pesquisada de cada corpo

# Words past this are not found. More takes more disk.
t0bf65f4a = Palavras além disto não são encontradas. Mais ocupa mais disco.

# Under heavy traffic
ta99888cb = Com tráfego intenso

# Memory for bodies waiting to be written
t7d6ff799 = Memória para corpos esperando gravação

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Acima disto, os fluxos são listados sem os corpos. O tráfego nunca é retido.

# Memory for large bodies waiting to be written
t07b9de08 = Memória para corpos grandes esperando gravação

# Past this, large bodies stop being kept.
tfe61a4f3 = Acima disto, os corpos grandes deixam de ser guardados.

# {key}: "{text}" is not a whole number
ta8606963 = { $key }: "{ $text }" não é um número inteiro

# {key}: must not be negative
t9c4debaa = { $key }: não pode ser negativo

# After a restart
t58dcea2d = Após reiniciar

# Every setting is back to its default.
t763785e7 = Todas as configurações voltaram ao padrão.

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d =
    { $n ->
        [one] { $n } alteração vale quando o tracewing for reiniciado.
        [many] { $n } alterações valem quando o tracewing for reiniciado.
       *[other] { $n } alterações valem quando o tracewing for reiniciado.
    }

# Settings
t4b058728 = Configurações

# Undo
t71fd4acf = Desfazer

# Back to defaults
ta3162a13 = Restaurar padrões

# Hosts not to decrypt
tb457f0b3 = Hosts a não descriptografar

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = Um por linha: um host, ou *.example.com para ele e tudo abaixo dele. O tráfego deles passa sem ser lido. Esta lista vale na hora.

# Language
t9a73db9b = Idioma

# System ({system_name})
tb5c7a1d4 = Sistema ({ $system_name })

# Applies at once.
tefb7a0f9 = Vale na hora.

# Your browser
tb5a48dd2 = Seu navegador

# Its private key is in the system keychain.
t62a47808 = A chave privada está no chaveiro do sistema.

# Its private key is in ca.key, which only you can read.
td9c6165d = A chave privada está em ca.key, que só você pode ler.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = A chave privada está em ca.key, que só você pode ler, pois o chaveiro do sistema não a aceita: { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Abra a página. Se o navegador avisar sobre o certificado, ou não encontrar a página, um dos passos acima não foi feito para esse navegador.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } passa pelo tracewing e confia nele.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } passa pelo tracewing, mas abriu a página sem HTTPS. Abra de novo por https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = Um celular ou outro computador também pode enviar o tráfego dele para cá. Ative “Permitir que outros dispositivos usem o proxy” em Configurações.

# this computer's address, port {port}
t2188b3f4 = o endereço deste computador, porta { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = No dispositivo, defina o proxy do Wi-Fi como { $addr }. Depois abra http://{ $host } nele, baixe o certificado e confie nele. Os fluxos dele mostram o endereço dele e podem ser filtrados com device:. Dê um nome a ele a partir de um deles, em Conexão.

# Looking for it…
t50ee7452 = Procurando…

# Installing. The system may ask you to confirm.
ta41c38ec = Instalando. O sistema pode pedir confirmação.

# Removing. The system may ask you to confirm.
t62b22726 = Removendo. O sistema pode pedir confirmação.

# This computer trusts it.
t27d6bacc = Este computador confia nele.

# Installed in: {places}.
tb5bcc244 = Instalado em: { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = As ferramentas de linha de comando confiam nele, mas os navegadores mantêm repositórios próprios. Até ele estar neles, o HTTPS pelo tracewing falha no navegador.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = O tracewing lê HTTPS com um certificado feito só para este computador. Até ele ser confiável, o HTTPS pelo tracewing falha.

# Trust the certificate
t26618699 = Confiar no certificado

# Install
tb50cbd40 = Instalar

# Also for command-line tools
t0db8faea = Também para ferramentas de linha de comando

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Remover o certificado de todos os repositórios deste computador? HTTPS pelo tracewing vai falhar até ele ser instalado de novo. O tracewing o guarda para instalar de novo depois.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = No Linux, os navegadores precisam do certutil para serem instruídos a confiar nele. Instale-o e depois clique em Instalar aqui:

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = No Linux, os navegadores precisam do certutil para serem instruídos a confiar nele. Instale as ferramentas NSS da sua distribuição e depois clique em Instalar aqui.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = Este desktop não tem uma configuração de proxy do sistema que o tracewing possa mudar. Abra um navegador à parte em Navegadores, ou um terminal, abaixo.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } não tem uma configuração de proxy do sistema que o tracewing possa mudar. Abra um navegador à parte em Navegadores, ou um terminal, abaixo.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Apps que usam a configuração do sistema agora passam por { $addr }. Ela volta ao que era quando o tracewing fecha, ou se ele travar.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Envia o tráfego deste computador pelo tracewing, para apps que usam a configuração do sistema. Ou aponte você mesmo um app para { $addr }.

# Turn on the system proxy
t52b88441 = Ativar o proxy do sistema

# Turn off
tcd03e04b = Desativar

# Turn on
t65a0c5a7 = Ativar

# Check it in your browser
tbed6e83a = Verifique no navegador

# Open the test page
te4e36f65 = Abrir a página de teste

# Set up
t43c06abc = Configurar

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Três passos para ver o tráfego HTTPS deste computador.

# See traffic
t980eee79 = Ver tráfego

# the browsers' own stores
tddf6836c = os repositórios próprios dos navegadores

# Trusts the certificate.
tc5bba858 = Confia no certificado.

# Does not trust the certificate yet.
tf2a0b8c5 = Ainda não confia no certificado.

# Looking…
t284579ba = Procurando…

# Has no profile yet. Open it once, then look again.
t00e4c88d = Ainda não tem perfil. Abra-o uma vez e procure de novo.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 =
    { $n ->
        [one] { $holding } de { $n } perfil confia no certificado.
        [many] { $holding } de { $n } perfis confiam no certificado.
       *[other] { $holding } de { $n } perfis confiam no certificado.
    }

# Has not opened the test page yet.
t67f9cb49 = Ainda não abriu a página de teste.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = Abriu a página de teste pelo tracewing, por HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = Abriu a página de teste pelo tracewing, mas sem HTTPS.

# Browsers
td580efd4 = Navegadores

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Cada um decide sozinho em que confiar e para onde mandar seu tráfego. Verificar abre nele a página de teste. Abrir à parte o inicia com um perfil próprio que passa pelo tracewing, sem mexer no proxy do sistema.

# Look again
t98416222 = Procurar de novo

# No browser found on this computer.
t0e46bb34 = Nenhum navegador encontrado neste computador.

# Check
t592c8de7 = Verificar

# Open on its own
t0da7783c = Abrir à parte

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = A porta UDP 443 está bloqueada neste computador, então os apps recuam do HTTP/3 para o HTTP/2 e passam pelo tracewing. O bloqueio para quando o tracewing fecha, ou se ele travar.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Alguns apps falam HTTP/3, sobre UDP, e contornam o proxy. Bloquear a porta UDP 443 neste computador faz com que recuem para o HTTP/2 pelo tracewing. Exige a senha de um administrador, e para quando o tracewing fecha. Uma VPN que usa a porta UDP 443 para de funcionar enquanto isso.

# Apps on HTTP/3
t2cfe5270 = Apps em HTTP/3

# Stop blocking
t318358d8 = Parar de bloquear

# Block HTTP/3
t88231e40 = Bloquear HTTP/3

# Or one app at a time
t91c63f1c = Ou um app de cada vez

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Um terminal que passa pelo tracewing, enquanto o proxy do sistema e os outros apps ficam como estão. Para um navegador, Abrir à parte em Navegadores faz o mesmo com um perfil próprio.

# Open a terminal
t4b629c72 = Abrir um terminal

# iPhone and iPad
ta550cc97 = iPhone e iPad

# Flutter apps
ta04ee397 = Apps Flutter

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. Em Ajustes, Wi-Fi, escolha a rede, depois Configurar Proxy, Manual: servidor { $server }, porta { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Leia o código com a câmera, ou abra { $page } no Safari, e baixe o certificado. Permita o download.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. Em Ajustes, Geral, VPN e Gerenciamento de Dispositivos, escolha o perfil tracewing CA e instale-o.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. Em Ajustes, Geral, Sobre, Ajustes de Confiança de Certificados, ative a confiança total para tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Abra https://{ $host } no Safari. Se disser que funciona, os apps que usam os ajustes do sistema passam pelo tracewing.

# When you are done
t62466f08 = Quando terminar

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. Em Ajustes, Wi-Fi, escolha a rede e depois Configurar Proxy: Desativado. Se ficar ligado, o aparelho fica sem Internet nesta rede quando o tracewing fechar.

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. Em Ajustes, Geral, VPN e Gerenciamento de Dispositivos, escolha tracewing CA e remova o perfil. A confiança total vai junto.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. Em Configurações, Wi-Fi, edite a rede e, em Opções avançadas, defina o proxy como Manual: host { $host }, porta { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Leia o código, ou abra { $page } no navegador, e baixe o certificado.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. Em Configurações, Segurança, Criptografia e credenciais (os nomes variam conforme o fabricante), escolha Instalar um certificado, depois Certificado CA, e selecione tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Desde o Android 7, os apps só confiam em certificados adicionados pelo usuário se declararem isso. Para um build de debug do seu próprio app, adicione res/xml/network_security_config.xml:

# and point to it from AndroidManifest.xml:
t02ad844b = e aponte para ele no AndroidManifest.xml:

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Outros apps continuam recusando o certificado: o tráfego deles aparece como fluxos com falha.

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. Em Configurações, Wi-Fi, edite a rede e volte o proxy para Nenhum. Se ficar ligado, o aparelho fica sem Internet nesta rede quando o tracewing fechar.

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. Em Configurações, Segurança, Criptografia e credenciais, Credenciais do usuário, escolha tracewing e remova. Apague também tracewing.crt de Downloads.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = O HttpClient do Dart, que o http e o Dio do Flutter usam, não segue a configuração de proxy do celular e, no Android, não confia em certificados adicionados pelo usuário. Em um build de debug, envie-o para cá:

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback deixa passar qualquer certificado, por isso é só para builds de debug.

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = Tire TracewingProxy e a linha dele em main() antes de publicar, embora não façam nada fora de builds de depuração. Nada mudou no telefone.

# Open settings
t1336b4d6 = Abrir configurações

# Phones and other devices
t2e3c591c = Celulares e outros dispositivos

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Um celular, um tablet ou outro computador nesta rede pode passar pelo tracewing em { $proxy }. O código abre a página com o certificado. Os fluxos dele mostram o endereço dele, podem ser filtrados com device: e podem receber um nome a partir de um deles, em Conexão.

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = Outros dispositivos não conseguem chegar ao tracewing em um dos endereços deste computador: { $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f =
    { $n ->
        [one] { $n } aparelho passou pelo tracewing ({ $names }). Quando terminar, desligue o proxy Wi-Fi dele, como abaixo: se ficar ligado, ele fica sem Internet nesta rede quando o tracewing fechar.
        [many] { $n } aparelhos passaram pelo tracewing ({ $names }). Quando terminar, desligue o proxy Wi-Fi deles, como abaixo: se ficar ligado, eles ficam sem Internet nesta rede quando o tracewing fechar.
       *[other] { $n } aparelhos passaram pelo tracewing ({ $names }). Quando terminar, desligue o proxy Wi-Fi deles, como abaixo: se ficar ligado, eles ficam sem Internet nesta rede quando o tracewing fechar.
    }

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Configurado. O certificado fica entre os do sistema até o dispositivo reiniciar.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Configurado. O certificado está em Downloads: nas Configurações que acabaram de abrir, escolha Criptografia e credenciais, Instalar um certificado, Certificado CA, e selecione tracewing.crt.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Apps que confiam só nos certificados do sistema ainda vão recusá-lo.

# An emulator.
t14b6b86d = Um emulador.

# On USB.
t67dafc14 = Por USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Aceite o aviso de depuração USB no dispositivo e procure de novo.

# Still starting. Look again in a moment.
t13f68b64 = Ainda iniciando. Procure de novo em instantes.

# adb says it is {state}.
t2b7d71da = O adb diz que está { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Define o proxy em um emulador Android, ou em um celular conectado com a depuração USB ativada, e instala o certificado. O tráfego vem pelo ADB, então nenhum dos dois precisa estar nesta rede.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = O adb não foi encontrado. Instale o Android Studio, ou as platform tools do Android, e procure de novo.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Nenhum dispositivo conectado. Inicie um emulador, ou conecte um celular com a depuração USB ativada, e procure de novo.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Configure um dispositivo para enviar o tráfego dele para cá. Desfazer, ou fechar o tracewing, tira o proxy dele de novo.

# Android emulators and USB devices
t379e433c = Emuladores Android e dispositivos USB

# Find devices
t54d4af39 = Encontrar dispositivos

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = O proxy do sistema e o bloqueio de HTTP/3 voltam ao que eram quando o tracewing fecha ou trava, assim como um aparelho Android configurado por USB. O certificado continua confiável até que Remover, no passo 1, o tire. Um telefone configurado à mão mantém o proxy Wi-Fi e o certificado até que sejam removidos nele: o guia de cada aparelho diz como. Outros aparelhos podem usar o proxy deste computador até que isso seja desligado nas Configurações.

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code e outras ferramentas de IA podem listar e ler os fluxos que o tracewing grava, pelo servidor MCP dele. Tokens e cookies ficam de fora, a menos que as Configurações permitam. No Claude Code:

# Elsewhere, in the tool's MCP settings:
tec38833c = Em outros lugares, nas configurações de MCP da ferramenta:

# Flows
t4db77db2 = Fluxos

# Choose the flow above or below
tecb3adec = Escolher o fluxo acima ou abaixo

# The first or the last flow
t7e278a00 = O primeiro ou o último fluxo

# A page up or down
t23b1814b = Uma página acima ou abaixo

# Filter
tf4a9c097 = Filtrar

# Pause or resume recording
t6e21519a = Pausar ou retomar a gravação

# Clear the list
tf2a55d1f = Limpar a lista

# The chosen flow
te0a729de = O fluxo escolhido

# Its request
tb3970784 = A requisição dele

# Its response
t221fc168 = A resposta dele

# Send it again
tef7d1944 = Reenviá-lo

# Edit it and send it
t6bc99e5c = Editá-lo e enviá-lo

# Copy it as cURL
tc99a11a3 = Copiá-lo como cURL

# The part before or after the one shown
tdf4221d9 = A parte antes ou depois da mostrada

# Save the body shown
t20d9b7c0 = Salvar o corpo mostrado

# Select all of the part shown
tf9168168 = Selecionar tudo na parte mostrada

# Copy the selected text
t00b090d3 = Copiar o texto selecionado

# Send it on, as edited
t8c35fa5a = Enviar como editado

# Send it on as it was
t16f7abb5 = Enviar como estava

# Drop it
t4e7f48b9 = Descartar

# The window
t2af18488 = A janela

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Configurar, Tráfego, Regras, Pausados, Compor, Sessões, Configurações

# Show or hide the sidebar
t67adca04 = Mostrar ou ocultar a barra lateral

# Show or hide the inspector
ta5a7b9e9 = Mostrar ou ocultar o inspetor

# The inspector beside the list or below it
t51e214f7 = O inspetor ao lado da lista ou abaixo dela

# The next or the previous field
ta4b4bd5e = O campo seguinte ou o anterior

# These shortcuts
tbdf3b62b = Estes atalhos

# Close them
tf3a4b1ef = Fechá-los

# Keyboard shortcuts
t31eaddb7 = Atalhos de teclado

# Close
tcd86acc3 = Fechar

# The certificate: {e}
t0235648c = O certificado: { $e }

# The terminal
tf542b2f4 = O terminal

# The system proxy: {e}
tc99726ce = O proxy do sistema: { $e }

# Blocking HTTP/3: {e}
t024ec871 = Bloqueio do HTTP/3: { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx, ou texto nos corpos

# A name, as iPhone test
t19f89214 = Um nome, como Teste iPhone

# The search index: {e}
te3c971c9 = O índice de pesquisa: { $e }

# An earlier session: {name}
t73446a1d = Uma sessão anterior: { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Somente leitura. A pesquisa cobre mais conforme o índice é concluído. A gravação continua.

# Read-only. Recording goes on.
te2f35d9e = Somente leitura. A gravação continua.

# Back to live
t317c0453 = Voltar ao vivo

# {shown} of {total}
tf2cc0563 = { $shown } de { $total }

# Recording
te75cf20c = Gravando

# System proxy off · {addr}
t7e813d25 = Proxy do sistema desativado · { $addr }

# System proxy…
t29e7dcd0 = Proxy do sistema…

# System proxy on · {addr}
t319ad6b9 = Proxy do sistema ativado · { $addr }

# Apps
t53d91aad = Apps

# Devices
t27acf320 = Dispositivos

# All hosts
tfc7bfe37 = Todos os hosts

# No host has this in its name
t838c1d44 = Nenhum host tem isto no nome

# Hidden
t3c2dfe59 = Ocultos

# Hosts
tcf9aefb4 = Hosts

# Newest
t324b6d0b = Mais recentes

# No flow fits this filter
t7eb295f4 = Nenhum fluxo corresponde a este filtro

# Nothing yet
tbd00747c = Nada ainda

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Ative o proxy do sistema acima, ou aponte um app para { $addr }. Para testar: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = Ao lado da lista

# Below the list
t23658daf = Abaixo da lista

# Drop it on a place · Esc puts it back
t0fa632ba = Solte em um lugar · Esc o devolve

# Traffic
t4b589266 = Tráfego

# Certificate trusted on this computer
t7b93f551 = Certificado confiável neste computador

# Checking the certificate…
tbf8942c3 = Verificando o certificado…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Instalando o certificado; o sistema pode pedir confirmação…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Removendo o certificado; o sistema pode pedir confirmação…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = O HTTPS vai falhar até o certificado ser confiável: configurar

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml não foi usado: { $problem }

# A setting was not saved: {problem}
tff1a5018 = Uma configuração não foi salva: { $problem }

# The proxy stopped: {failure}
t04dcff11 = O proxy parou: { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = O disco está quase cheio: os corpos não estão sendo guardados

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
        [one] { $count } fluxo não gravado
       *[other] { $count } fluxos não gravados
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
        [one] { $count } fluxo sem corpo
       *[other] { $count } fluxos sem corpos
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
        [one] { $count } fluxo mais antigo descartado para ficar abaixo de { $size }
       *[other] { $count } fluxos mais antigos descartados para ficar abaixo de { $size }
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
        [one] { $count } fluxo
       *[other] { $count } fluxos
    }

# Export as HAR
t95b6b766 = Exportar como HAR

# Shortcuts {keys}
t4ab0ecb4 = Atalhos { $keys }

# Columns
tea156fdc = Colunas
