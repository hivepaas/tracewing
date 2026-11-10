# tracewing in French. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 =
    { $n ->
        [one] … { $count } ligne { $closer }
        [many] … { $count } lignes { $closer }
       *[other] … { $count } lignes { $closer }
    }

# … {size} more
ta8dc6eab = … { $size } de plus

# Unfold all
t3849d44e = Tout déplier

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
        [one] { $size } · { $count } ligne
       *[other] { $size } · { $count } lignes
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · lecture { $percent } %

# Appearance: {now}. Click for {next}.
t332cfa6d = Apparence : { $now }. Cliquez pour choisir { $next }.

# Method
t98a2c750 = Méthode

# Status
t005ef20f = Statut

# App
t81f51acc = App

# Host
t302af18f = Hôte

# Path
teb66e456 = Chemin

# Size
ta6478e7c = Taille

# Time
tdfe4e404 = Temps

# Number
t61e9eac0 = Numéro

# "{method}" is not a method
tacdcf5cb = « { $method } » n'est pas une méthode

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = le corps de sa requête n'a pas été conservé en entier, elle ne peut donc pas être renvoyée

# name: value, one a line
t4df37bea = nom: valeur, un par ligne

# No body
tfa4c2902 = Aucun corps

# No flow is chosen
t550b0a98 = Aucun flow sélectionné

# This flow: {e}
t296762f8 = Ce flow : { $e }

# Request
t82454152 = Requête

# Headers
t334f5989 = En-têtes

# Body
t428a1095 = Corps

# Sending…
te946e7bf = Envoi…

# Send
tf28e14cf = Envoyer

# New
t8b242db1 = Nouvelle

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … { $count } octets de plus : le flow dans Trafic les contient tous.

# Response
t55a982fe = Réponse

# Compose
t18049f93 = Composer

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = Une requête à vous, ou une requête renvoyée. Elle passe par tracewing : les règles s'appliquent et elle apparaît aussi dans Trafic.

# {version} both sides
t961a5ea0 = { $version } des deux côtés

# {client} to the proxy, {server} to the server
t3d237646 = { $client } vers le proxy, { $server } vers le serveur

# Queued
t9e437c74 = En file d'attente

# Connect
tdb89a899 = Connexion

# Waiting
tc9450e48 = Attente

# Download
t6964bb19 = Téléchargement

# No file picker: {e}
t6935fbd3 = Aucun sélecteur de fichiers : { $e }

# Writing the HAR…
t725a0e89 = Écriture du HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
        [one] { $n } flow écrit dans { $path }
       *[other] { $n } flows écrits dans { $path }
    }

# The HAR was not written: {e}
tef8ec569 = Le HAR n'a pas été écrit : { $e }

# Open
t538b10e9 = Ouvrir

# Reading {path}…
tc0e7c8f4 = Lecture de { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
        [one] , { $n } entrée ignorée
       *[other] , { $n } entrées ignorées
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
        [one] { $n } flow lu depuis une { $format }
       *[other] { $n } flows lus depuis une { $format }
    }

# The file was not read: {e}
t7f49510f = Le fichier n'a pas été lu : { $e }

# it is no longer kept
ta7b4c380 = il n'est plus conservé

# {size} saved to {path}
tc6932467 = { $size } enregistrés dans { $path }

# The body was not saved: {e}
t189900b4 = Le corps n'a pas été enregistré : { $e }

# Fiddler session
t9f5c89a3 = session Fiddler

# Charles session
tb4f56f2b = session Charles

# All
t75afe364 = Tous

# Errors
te9ed60a6 = Erreurs

# Media
td10fcaa3 = Médias

# Rule: {rule}
teca2074f = Règle : { $rule }

# Events
t003420a4 = Événements

# Not decrypted
t8090a1ba = Non déchiffré

# open
td35ec4c9 = ouvert

# Cut
t58e25863 = Couper

# Copy
t658f3664 = Copier

# Paste
tb34a0288 = Coller

# Select all
t6af21072 = Tout sélectionner

# This flow is gone.
t3343999e = Ce flow n'existe plus.

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = Cet hôte n'est pas déchiffré, on ne peut donc pas voir ce qu'il envoie.

# No body.
t31e4d044 = Aucun corps.

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = Ce corps n'a pas été conservé : le disque était presque plein, ou trop de données attendaient d'être écrites.

# This body was not kept.
te46616b5 = Ce corps n'a pas été conservé.

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = Ce corps n'existe plus : il a été supprimé avec les flows les plus anciens, ou ne peut pas être lu.

# Cannot open this body: {e}
t620a039b = Impossible d'ouvrir ce corps : { $e }

# Inspector
t4def410e = Inspecteur

# Choose a flow to see it here
tc7be91e8 = Choisissez un flow pour le voir ici

# Not decrypted. New connections to this host go through unread.
ta05ec37a = Non déchiffré. Les nouvelles connexions vers cet hôte passent sans être lues.

# If an app fails with this host, it may pin its certificates.
t3cbba118 = Si une app échoue avec cet hôte, elle épingle peut-être ses certificats.

# Make a rule
t0ac0e062 = Créer une règle

# Decrypt this host
tb7367e2c = Déchiffrer cet hôte

# Do not decrypt this host
t5fa113f6 = Ne pas déchiffrer cet hôte

# Request and response
t04e060e6 = Requête et réponse

# Request and response one at a time
t70397119 = Requête et réponse, l’une après l’autre

# Request and response side by side
t3d35ce69 = Requête et réponse côte à côte

# Request above the response
t83c63123 = Requête au-dessus de la réponse

# Picture · {size}
te97ed633 = Image · { $size }

# Messages {count}
te61b841a = Messages { $count }

# Headers {headers}
t04b0146d = En-têtes { $headers }

# Cookies {cookies}
td58eb84b = Cookies { $cookies }

# Query {query}
t4756f9f5 = Query { $query }

# Form {form}
tfa6e7203 = Formulaire { $form }

# Connection
t09ea4ff9 = Connexion

# Save body
te2c44424 = Enregistrer le corps

# Send again
tf5910557 = Renvoyer

# Edit and send
tf837e542 = Modifier et envoyer

# Copied
t8dc21305 = Copié

# Copy as cURL
t866d1e3e = Copier en cURL

# Copy as…
t9c1ad448 = Copier en…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = Cette image ne peut pas être affichée. Enregistrez le corps pour la voir.

# Opening…
tc40f2321 = Ouverture…

# No response
t8c2c37b1 = Aucune réponse

# Tunnel, not decrypted
tce693254 = Tunnel, non déchiffré

# GraphQL: {operation}
t2d65757d = GraphQL : { $operation }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = Via la connexion { $id }, établie auparavant : aucun temps passé à se connecter.

# {app}, pid {pid}
t627147b4 = { $app }, pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = Introuvable : il s'est peut-être terminé, ou appartient à un autre utilisateur

# On another device
t2b964a37 = Sur un autre appareil

# CLIENT
t6d3cd99e = CLIENT

# Device
tbdcbb413 = Appareil

# Program
tc4253b6b = Programme

# Address
t700560f3 = Adresse

# SERVER
te3b27612 = SERVEUR

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = Les informations sur cette connexion ne sont plus conservées : elles ont été supprimées avec des flows plus anciens.

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = Non déchiffrée : tracewing l'a transmise sans la lire.

# No connection was made.
tf348eb92 = Aucune connexion n'a été établie.

# None: it was answered without one.
tac71808e = Aucune : la réponse a été donnée sans connexion.

# {id}, given up during TLS
t0ec66a8c = { $id }, abandonnée pendant TLS

# {id}, made for this flow
t7fe32cfc = { $id }, établie pour ce flow

# {id}, made before and used again
ta904c078 = { $id }, établie auparavant et réutilisée

# Server
t6d7302f2 = Serveur

# None: plain HTTP
t05606349 = Aucun : HTTP en clair

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = LE SERVEUR N'A PRÉSENTÉ AUCUN CERTIFICAT

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = CE QUE LE SERVEUR A PRÉSENTÉ, LE SIEN EN PREMIER

# NO CERTIFICATES
te6f31dce = AUCUN CERTIFICAT

# CERTIFICATE
t1c902790 = CERTIFICAT

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
        [one] { $n } CERTIFICAT, CELUI DU SERVEUR EN PREMIER
       *[other] { $n } CERTIFICATS, CELUI DU SERVEUR EN PREMIER
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing a refusé de continuer avec ce serveur : { $error }. Si l'app lui fait confiance autrement, par exemple via une CA d'entreprise, choisissez « Ne pas déchiffrer cet hôte » et l'app vérifiera le serveur elle-même.

# Cipher
t5b3bed6a = Chiffrement

# none
tada7afdb = aucun

# not valid until {when}
t0a14b2d8 = non valide avant { $when }

# expired {when}
t932538c2 = expiré { $when }

# expires {when}
t679bcb51 = expire { $when }

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
        [one] { $names } et { $n } autre
       *[other] { $names } et { $n } autres
    }

# Issued by {issuer}
t15a1ce5a = Émis par { $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = Valide du { $from } au { $to } · { $when }

# Names {names}
tda75e459 = Noms { $names }

# Serial {serial}
td1f0d98b = Numéro de série { $serial }

# These headers were not kept.
tb8739fec = Ces en-têtes n'ont pas été conservés.

# No headers.
tf3dacb5e = Aucun en-tête.

# File {filename} · {kind} · {size}
t6dbd239f = Fichier { $filename } · { $kind } · { $size }

# no type
t902a6ece = aucun type

# No cookies were sent.
t58e85ab2 = Aucun cookie n'a été envoyé.

# No cookies were set.
tf57dbabc = Aucun cookie n'a été défini.

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · signature non vérifiée

# JWT · signature not checked
t05f8b533 = JWT · signature non vérifiée

# Header
t114f9980 = En-tête

# Payload
ta0639745 = Charge utile

# Expires
tc7aa0523 = Expire

# Issued
tb1835ecc = Émis

# Not before
td10dacd9 = Pas avant

# in {amount}
tfda57c3e = dans { $amount }

# {amount} ago
t5f968014 = il y a { $amount }

# Light
t8ad2f8cf = Clair

# Dark
t45570065 = Sombre

# System
t8f3184bc = Système

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
        [one] diffusé en { $count } morceau
       *[other] diffusé en { $count } morceaux
    }

# stopped: {stop}
tfa48fb30 = arrêté : { $stop }

# {input} in · {output} out tokens
tc8bbc724 = jetons : { $input } en entrée · { $output } en sortie

# Reading the call…
t33b52a3c = Lecture de l'appel…

# Answer
tf94ea0ff = Réponse

# Thinking: {thinking}
t737ece33 = Réflexion : { $thinking }

# Calls {name}{id}
t017882e4 = Appelle { $name }{ $id }

# Nothing yet.
td9b71d16 = Rien pour l'instant.

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
        [one] { $text }… et { $count } caractère de plus
       *[other] { $text }… et { $count } caractères de plus
    }

# Copy the answer
t69975ed7 = Copier la réponse

# Asked
t4e3af9c3 = Question

# Tools offered: {tools}
tb871bcf4 = Outils proposés : { $tools }

# Not kept
tba30b5fa = Non conservé

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = Ce message n'a pas été conservé : trop de données attendaient, ou le disque était presque plein.

# This message is empty.
tdb0ec01f = Ce message est vide.

# No messages yet
t51e7624c = Aucun message pour l'instant

# {said} kept, of {size}
t3558745a = { $said } conservés sur { $size }

# {what}: {text} is not a whole number
t2a8515ce = { $what } : { $text } n'est pas un nombre entier

# Latency
t23a45ef9 = Latence

# Down
ta4995035 = Descendant

# Up
t42f35940 = Montant

# Loss
t69db303a = Perte

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = Indiquez une latence, un débit ou une perte pour ralentir le trafic

# Latency, ms
tefe18ab9 = Latence, ms

# Down, kbit/s
t08c4ae33 = Descendant, kbit/s

# Up, kbit/s
td55cfe9a = Montant, kbit/s

# Loss, %
tea4af439 = Perte, %

# Only these hosts, like *.example.com; all if empty
t77a1f991 = Uniquement ces hôtes, comme *.example.com ; tous si vide

# Not kept for next time: {e}
tdce4d67c = Non conservé pour la prochaine fois : { $e }

# Slow network
td51ddfce = Réseau lent

# Off: traffic goes as fast as it can
tf0f34db0 = Désactivé : le trafic va aussi vite que possible

# Custom
t9525b78e = Personnalisé

# Only these hosts
tfe16992b = Uniquement ces hôtes

# Apply
tca77bbdb = Appliquer

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = S'applique à partir de la prochaine requête. La latence s'ajoute à chaque aller-retour ; le débit est un lien partagé par tous les flows ralentis ; un morceau perdu attend d'être renvoyé, comme avec TCP. Les autres hôtes ne sont pas ralentis.

# Header line {line} has no ":"
t891b076d = La ligne d'en-tête { $line } n'a pas de « : »

# Header line {line} has no name
te3247b96 = La ligne d'en-tête { $line } n'a pas de nom

# URL
t7569633e = URL

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
        [one] { $n } octet qui n'est pas du texte : il est transmis tel quel.
       *[other] { $n } octets qui ne sont pas du texte : ils sont transmis tels quels.
    }

# Nothing is paused
t8cf68b4c = Rien n'est en pause

# "{status}" is not a status
t40a529af = « { $status } » n'est pas un statut

# REQUEST
t1f3f6dd2 = REQUÊTE

# RESPONSE
tc85f77de = RÉPONSE

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = Rien n'est en pause. Ajoutez une règle de pause dans l'écran Règles pour arrêter le trafic ici.

# for {method} {url}
t093a86f6 = pour { $method } { $url }

# The request, before the server gets it
tc840fed6 = La requête, avant que le serveur ne la reçoive

# The response, before the app gets it
t6683d72a = La réponse, avant que l'app ne la reçoive

# Send as it came
tb593e730 = Envoyer tel quel

# Drop
t108524ac = Rejeter

# Paused
tde281feb = En pause

# Nothing is waiting.
tea613769 = Rien n'attend.

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
        [one] { $n } attend d'être transmis.
       *[other] { $n } attendent d'être transmis.
    }

# Send all as they came
tdce25640 = Tout envoyer tel quel

# Block
tbd074002 = Bloquer

# Answer from a file
td8edab50 = Répondre avec un fichier

# Send to another host
te3cd85d7 = Envoyer vers un autre hôte

# Set a request header
tfad4aaaa = Définir un en-tête de requête

# Set a response header
tcbbc20d0 = Définir un en-tête de réponse

# Remove a request header
t5228dd84 = Retirer un en-tête de requête

# Remove a response header
teeb5735a = Retirer un en-tête de réponse

# Set a query parameter
t73d9419b = Définir un paramètre de requête

# Remove a query parameter
t12633da9 = Retirer un paramètre de requête

# Change the request body
t41101201 = Changer le corps de la requête

# Change the response body
tdd259b73 = Changer le corps de la réponse

# Change the status
t4bbdfba8 = Changer le statut

# Pause the request
t40ecc39b = Mettre la requête en pause

# Pause the response
t18a34799 = Mettre la réponse en pause

# Status to answer with, like 403
t8f3ebb6c = Statut de la réponse, comme 403

# Full path of a file or folder
t9fb9ae4c = Chemin complet d'un fichier ou d'un dossier

# Where to, like http://localhost:3000
tdfedd004 = Destination, comme http://localhost:3000

# Header's name
t80793085 = Nom de l'en-tête

# Its value
t43968778 = Sa valeur

# Parameter's name
t313c8e35 = Nom du paramètre

# Text to find, or empty for the whole body
t8255fc87 = Texte à chercher, ou vide pour tout le corps

# What to put in its place
t03671fd5 = Ce qui prend sa place

# Status the client gets, like 500
t5e4d3088 = Statut que reçoit le client, comme 500

# "{first}" is not a status
td43128d3 = « { $first } » n'est pas un statut

# Block with {status}
t1727e984 = Bloquer avec { $status }

# Answer from {path}
taefc08fa = Répondre avec { $path }

# Send to {to}
t5e256873 = Envoyer vers { $to }

# Request: {header}: {value}
taa230766 = Requête : { $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = Réponse : { $header }: { $value }

# Request: no {header}
t6f0b3f5a = Requête : sans { $header }

# Response: no {header}
t23960a2e = Réponse : sans { $header }

# Query: {name}={value}
t0ee6c1c0 = Query : { $name }={ $value }

# Query: no {name}
tf2e7bf27 = Query : sans { $name }

# Status {status}
tad048e53 = Statut { $status }

# Request body: {replace}
te4856c36 = Corps de la requête : { $replace }

# Response body: {replace}
t8600dc42 = Corps de la réponse : { $replace }

# Request body: {find} → {replace}
taec471a9 = Corps de la requête : { $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = Corps de la réponse : { $find } → { $replace }

# Name
t0fe07306 = Nom

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL, comme api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = Méthode, ou vide pour toutes

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
        [one] { $n } règle écrite dans { $path }
       *[other] { $n } règles écrites dans { $path }
    }

# The rules were not written: {e}
te7ac257f = Les règles n'ont pas été écrites : { $e }

# Import
t573e0af4 = Importer

# {path} could not be read: {e}
t32a3ce94 = Impossible de lire { $path } : { $e }

# Nothing was imported: {e}
tb9f7257a = Rien n'a été importé : { $e }

# Edit
tc2c76cb1 = Modifier

# Delete
t5797ea6a = Supprimer

# Change this rule
t9160f393 = Modifier cette règle

# A new rule
t55a722a0 = Nouvelle règle

# It waits under Paused until you send it on, changed or not.
tca566a57 = Elle attend dans En pause jusqu'à ce que vous la transmettiez, modifiée ou non.

# Save
t4d2d5d68 = Enregistrer

# Add
t9dc3aa14 = Ajouter

# Cancel
t35afca3b = Annuler

# Rules
t428b2d80 = Règles

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = Elles s’appliquent dès la requête suivante, même enregistrement en pause. De haut en bas : le premier blocage, fichier ou hôte l’emporte, et toute autre règle qui convient s’applique, dans l’ordre. Une règle qui change un corps ou un statut change ce que le serveur a envoyé.

# Import rules…
t5edc205d = Importer des règles…

# Export rules…
t6e09ca20 = Exporter les règles…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
        [one] { $n } règle ajoutée
       *[other] { $n } règles ajoutées
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
        [one] { $n } déjà présente
       *[other] { $n } déjà présentes
    }

# left out: {why}
ta81e95e2 = ignorée : { $why }

# Copy all
t6aef5c37 = Tout copier

# Too large to copy all
t2fe8ff9f = Trop volumineux pour tout copier

# No flows
t6dc463eb = Aucun flow

# Name it to keep it
t52c1a402 = Nommez-la pour la conserver

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · écrite par une autre version, impossible de l'ouvrir

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
        [one] { $count } flow · { $size }
       *[other] { $count } flows · { $size }
    }

# Recording now
t3cb3e6fc = Capture en cours

# Export HAR
t4d6458cc = Exporter en HAR

# Delete for good?
ta7737f13 = Supprimer définitivement ?

# Sessions
tb69a8aec = Sessions

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = Une session nommée est conservée jusqu'à ce que vous la supprimiez. Parmi les autres, les { $kept } plus récentes sont gardées.

# Working…
td5507854 = Traitement…

# Open a session file
tae8622fc = Ouvrir un fichier de session

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR, .saz de Fiddler ou .chlsj de Charles

# Exported HAR files leave out tokens and cookies
td4a2f543 = Les fichiers HAR exportés omettent les jetons et les cookies

# Other devices
t9e7d8362 = Autres appareils

# Let other devices use the proxy
t13e626b3 = Autoriser les autres appareils à utiliser le proxy

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust. Takes effect when tracewing starts again.
t2e14229d = Les téléphones et autres ordinateurs de votre réseau peuvent alors envoyer leur trafic ici. Tout autre utilisateur du réseau aussi : laissez cette option désactivée sur un réseau auquel vous ne faites pas confiance. Prend effet au prochain démarrage de tracewing.

# Servers
tcd0cf113 = Serveurs

# Trust what this computer trusts
tfea2d91b = Faire confiance à ce que cet ordinateur approuve

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = Les serveurs dont le certificat provient d'une autorité installée sur cet ordinateur, comme celle d'une entreprise, sont joints comme les joignent les navigateurs de cet ordinateur. Désactivé, seules les autorités publiques sont approuvées. Prend effet au prochain démarrage de tracewing.

# AI tools
t294d7ea4 = Outils d'IA

# Let AI tools see tokens and cookies
te1d26d3c = Laisser les outils d'IA voir les jetons et les cookies

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = Les outils d'IA lisent le trafic capturé via le serveur MCP de tracewing. Désactivé, les valeurs de Authorization, Cookie et Set-Cookie sont retirées de ce qu'ils reçoivent. S'applique immédiatement.

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = Tenir les apps à l'écart de HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them. Takes effect when tracewing starts again.
t2aa7be0d = HTTP/3 passe par UDP, qu'aucun proxy ne transporte, donc une app qui l'utilise contourne tracewing. Cette option retire des réponses les offres HTTP/3 des serveurs, et les apps restent en HTTP/2. Les flows montrent toujours les offres telles que les serveurs les ont envoyées. Prend effet au prochain démarrage de tracewing.

# Size of a session
ta20fa102 = Taille d'une session

# Past this, the oldest flows are dropped.
td2914f46 = Au-delà, les flows les plus anciens sont supprimés.

# Room to leave on the disk
t8674a3b3 = Espace à laisser libre sur le disque

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = Avec moins d'espace libre, les flows sont toujours listés mais les corps ne sont pas conservés.

# Earlier sessions to keep
td356194a = Sessions précédentes à conserver

# Older ones are deleted when a new session starts.
t66556513 = Les plus anciennes sont supprimées au début d'une nouvelle session.

# Bodies
tdb4d0409 = Corps

# Most kept of one body
t777508dd = Taille maximale conservée d'un corps

# It still reaches the client whole.
t386fee28 = Il parvient quand même entier au client.

# Most unpacked for viewing
tc3fe2efd = Taille maximale décompressée pour l'affichage

# Stops a small download that unpacks to many GB.
tf9c49515 = Empêche un petit téléchargement de se décompresser en plusieurs Go.

# Small body size
ta8996af1 = Taille d'un petit corps

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = Les corps plus petits attendent en mémoire et partagent un fichier. Les plus grands ont leur propre fichier dès leur arrivée.

# Compress plain text on disk
tee503c2e = Compresser le texte brut sur le disque

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = Prend environ moitié moins de place, et est ignoré quand le trafic est trop rapide pour cela.

# Compression level
td386c9ef = Niveau de compression

# 1 to 19. Higher is smaller and slower.
td42a987d = 1 à 19. Plus élevé : plus petit et plus lent.

# Search
tc646a2c9 = Recherche

# Search URLs
t0c3e0b3d = Rechercher dans les URL

# Finds any part of a URL as you type. Costs little.
t3f22d666 = Trouve n'importe quelle partie d'une URL pendant la saisie. Peu coûteux.

# Search bodies
t8eec0293 = Rechercher dans les corps

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = Trouve des mots dans les corps. Peut occuper autant d'espace disque que les corps eux-mêmes.

# Searched part of each body
tab293b21 = Partie de chaque corps couverte par la recherche

# Words past this are not found. More takes more disk.
t0bf65f4a = Les mots au-delà ne sont pas trouvés. Plus prend plus d'espace disque.

# Under heavy traffic
ta99888cb = En cas de trafic intense

# Memory for bodies waiting to be written
t7d6ff799 = Mémoire pour les corps en attente d'écriture

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = Au-delà, les flows sont listés sans leurs corps. Le trafic n'est jamais retenu.

# Memory for large bodies waiting to be written
t07b9de08 = Mémoire pour les gros corps en attente d'écriture

# Past this, large bodies stop being kept.
tfe61a4f3 = Au-delà, les gros corps ne sont plus conservés.

# {key}: "{text}" is not a whole number
ta8606963 = { $key } : « { $text } » n'est pas un nombre entier

# {key}: must not be negative
t9c4debaa = { $key } : ne doit pas être négatif

# Saved. The limits apply the next time tracewing starts.
tc8bd2065 = Enregistré. Les limites s'appliquent au prochain démarrage de tracewing.

# Settings
t4b058728 = Paramètres

# Back to defaults
ta3162a13 = Rétablir les valeurs par défaut

# Hosts not to decrypt
tb457f0b3 = Hôtes à ne pas déchiffrer

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies as soon as it is saved.
t7647c310 = Un par ligne : un hôte, ou *.example.com pour lui et tout ce qui est en dessous. Leur trafic passe sans être lu. Cette liste s'applique dès qu'elle est enregistrée.

# Language
t9a73db9b = Langue

# System ({system_name})
tb5c7a1d4 = Système ({ $system_name })

# Applies at once.
tefb7a0f9 = S'applique immédiatement.

# Your browser
tb5a48dd2 = Votre navigateur

# Its private key is in the system keychain.
t62a47808 = Sa clé privée est dans le trousseau du système.

# Its private key is in ca.key, which only you can read.
td9c6165d = Sa clé privée est dans ca.key, que vous seul pouvez lire.

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = Sa clé privée est dans ca.key, que vous seul pouvez lire, car le trousseau du système ne peut pas l'accepter : { $why }.

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = Ouvrez la page. Si le navigateur affiche un avertissement sur le certificat, ou ne trouve pas la page, l'une des étapes ci-dessus n'est pas faite pour ce navigateur.

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } passe par tracewing et lui fait confiance.

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } passe par tracewing, mais a ouvert la page sans HTTPS. Rouvrez-la via https://.

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings, then start tracewing again.
t9f95b7b8 = Un téléphone ou un autre ordinateur peut aussi envoyer son trafic ici. Activez « Autoriser les autres appareils à utiliser le proxy » dans Paramètres, puis redémarrez tracewing.

# this computer's address, port {port}
t2188b3f4 = l'adresse de cet ordinateur, port { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = Sur l'appareil, définissez le proxy Wi-Fi sur { $addr }. Ouvrez ensuite http://{ $host } sur l'appareil, récupérez le certificat et faites-lui confiance. Ses flows affichent son adresse et peuvent être filtrés avec device:. Nommez-le depuis l'un d'eux, dans Connexion.

# Looking for it…
t50ee7452 = Recherche…

# Installing. The system may ask you to confirm.
ta41c38ec = Installation. Le système peut vous demander de confirmer.

# Removing. The system may ask you to confirm.
t62b22726 = Retrait en cours. Le système peut demander une confirmation.

# This computer trusts it.
t27d6bacc = Cet ordinateur lui fait confiance.

# Installed in: {places}.
tb5bcc244 = Installé dans : { $places }.

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = Les outils en ligne de commande lui font confiance, mais les navigateurs ont leurs propres magasins. Tant qu'il n'y est pas, HTTPS via tracewing échoue dans un navigateur.

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing lit HTTPS avec un certificat créé pour cet ordinateur uniquement. Tant qu'il n'est pas approuvé, HTTPS via tracewing échoue.

# Trust the certificate
t26618699 = Approuver le certificat

# Install
tb50cbd40 = Installer

# Also for command-line tools
t0db8faea = Aussi pour les outils en ligne de commande

# Remove
t21a5901d = Retirer

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = Retirer le certificat de tous les magasins de cet ordinateur ? HTTPS via tracewing échouera ensuite jusqu’à ce qu’il soit réinstallé. tracewing le garde pour le réinstaller plus tard.

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = Sous Linux, les navigateurs ont besoin de certutil pour qu’on leur dise de s’y fier. Installez-le, puis Installer ici :

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = Sous Linux, les navigateurs ont besoin de certutil pour qu’on leur dise de s’y fier. Installez les outils NSS de votre distribution, puis Installer ici.

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = Ce bureau n’a pas de réglage de proxy système que tracewing puisse changer. Ouvrez plus bas un navigateur à part depuis Navigateurs, ou un terminal.

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } n’a pas de réglage de proxy système que tracewing puisse changer. Ouvrez plus bas un navigateur à part depuis Navigateurs, ou un terminal.

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = Les apps qui utilisent le réglage du système passent maintenant par { $addr }. Il est rétabli quand tracewing se ferme, ou s'il plante.

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = Fait passer le trafic de cet ordinateur par tracewing, pour les apps qui utilisent le réglage du système. Ou dirigez vous-même une app vers { $addr }.

# Turn on the system proxy
t52b88441 = Activer le proxy système

# Turn off
tcd03e04b = Désactiver

# Turn on
t65a0c5a7 = Activer

# Check it in your browser
tbed6e83a = Vérifier dans votre navigateur

# Open the test page
te4e36f65 = Ouvrir la page de test

# Set up
t43c06abc = Configurer

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = Trois étapes pour voir le trafic HTTPS de cet ordinateur.

# See traffic
t980eee79 = Voir le trafic

# the browsers' own stores
tddf6836c = les magasins propres aux navigateurs

# Trusts the certificate.
tc5bba858 = Fait confiance au certificat.

# Does not trust the certificate yet.
tf2a0b8c5 = Ne fait pas encore confiance au certificat.

# Looking…
t284579ba = Recherche…

# Has no profile yet. Open it once, then look again.
t00e4c88d = N’a pas encore de profil. Ouvrez-le une fois, puis cherchez à nouveau.

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 =
    { $n ->
        [one] { $holding } de son { $n } profil fait confiance au certificat.
        [many] { $holding } de ses { $n } profils font confiance au certificat.
       *[other] { $holding } de ses { $n } profils font confiance au certificat.
    }

# Has not opened the test page yet.
t67f9cb49 = N’a pas encore ouvert la page de test.

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = A ouvert la page de test via tracewing, en HTTPS.

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = A ouvert la page de test via tracewing, mais sans HTTPS.

# Browsers
td580efd4 = Navigateurs

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = Chacun décide à sa façon à quoi se fier et où envoyer son trafic. Vérifier y ouvre la page de test. Ouvrir à part le lance avec un profil à lui qui passe par tracewing, sans toucher au proxy système.

# Look again
t98416222 = Rechercher à nouveau

# No browser found on this computer.
t0e46bb34 = Aucun navigateur trouvé sur cet ordinateur.

# Check
t592c8de7 = Vérifier

# Open on its own
t0da7783c = Ouvrir à part

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = Le port UDP 443 est bloqué sur cet ordinateur, donc les apps repassent de HTTP/3 à HTTP/2 et passent par tracewing. Le blocage s'arrête quand tracewing se ferme, ou s'il plante.

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = Certaines apps parlent HTTP/3, sur UDP, et contournent le proxy. Bloquer le port UDP 443 sur cet ordinateur les fait repasser en HTTP/2 via tracewing. Cela demande le mot de passe d'un administrateur, et s'arrête quand tracewing se ferme. Un VPN qui utilise le port UDP 443 cesse de fonctionner pendant ce temps.

# Apps on HTTP/3
t2cfe5270 = Apps en HTTP/3

# Stop blocking
t318358d8 = Arrêter le blocage

# Block HTTP/3
t88231e40 = Bloquer HTTP/3

# Or one app at a time
t91c63f1c = Ou une app à la fois

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = Un terminal qui passe par tracewing, le proxy système et les autres apps restant tels quels. Pour un navigateur, Ouvrir à part dans Navigateurs fait de même avec un profil à lui.

# Open a terminal
t4b629c72 = Ouvrir un terminal

# iPhone and iPad
ta550cc97 = iPhone et iPad

# Flutter apps
ta04ee397 = Apps Flutter

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. Dans Réglages, Wi-Fi, choisissez le réseau, puis Configurer le proxy, Manuel : serveur { $server }, port { $port }.

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. Scannez le code avec l'appareil photo, ou ouvrez { $page } dans Safari, et récupérez le certificat. Autorisez le téléchargement.

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. Dans Réglages, Général, VPN et gestion de l'appareil, choisissez le profil tracewing CA et installez-le.

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. Dans Réglages, Général, Informations, Réglages de confiance des certificats, activez la confiance totale pour tracewing CA.

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. Ouvrez https://{ $host } dans Safari. S'il indique que cela fonctionne, les apps qui utilisent les réglages du système passent par tracewing.

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. Dans Paramètres, Wi-Fi, modifiez le réseau, puis sous Options avancées réglez le proxy sur Manuel : hôte { $host }, port { $port }.

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. Scannez le code, ou ouvrez { $page } dans le navigateur, et récupérez le certificat.

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. Dans Paramètres, Sécurité, Chiffrement et identifiants (les noms varient selon le fabricant), choisissez Installer un certificat, puis Certificat CA, et sélectionnez tracewing.crt.

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. Depuis Android 7, les apps ne font confiance aux certificats ajoutés par l'utilisateur que si elles le déclarent. Pour une version de débogage de votre propre app, ajoutez res/xml/network_security_config.xml :

# and point to it from AndroidManifest.xml:
t02ad844b = et référencez-le depuis AndroidManifest.xml :

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = Les autres apps continuent de refuser le certificat : leur trafic apparaît comme des flows en échec.

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = Le HttpClient de Dart, qu'utilisent http et Dio dans Flutter, ne suit pas le réglage de proxy du téléphone et, sur Android, ne fait pas confiance aux certificats ajoutés par l'utilisateur. Dans une version de débogage, envoyez-le ici :

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback laisse passer n'importe quel certificat, c'est pourquoi il est réservé aux versions de débogage.

# Open settings
t1336b4d6 = Ouvrir les paramètres

# Phones and other devices
t2e3c591c = Téléphones et autres appareils

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = Un téléphone, une tablette ou un autre ordinateur de ce réseau peut passer par tracewing à { $proxy }. Le code ouvre la page avec son certificat. Ses flows affichent son adresse, peuvent être filtrés avec device: et l'appareil peut être nommé depuis l'un d'eux, dans Connexion.

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = Configuré. Le certificat fait partie de ceux du système jusqu'au redémarrage de l'appareil.

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = Configuré. Le certificat est dans Téléchargements : dans les Paramètres qui viennent de s'ouvrir, choisissez Chiffrement et identifiants, Installer un certificat, Certificat CA, et sélectionnez tracewing.crt.

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = Les apps qui ne font confiance qu'aux certificats du système le refuseront toujours.

# An emulator.
t14b6b86d = Un émulateur.

# On USB.
t67dafc14 = En USB.

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = Acceptez la demande de débogage USB sur l'appareil, puis recherchez à nouveau.

# Still starting. Look again in a moment.
t13f68b64 = Démarrage en cours. Recherchez à nouveau dans un instant.

# adb says it is {state}.
t2b7d71da = adb indique l'état { $state }.

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = Configure le proxy sur un émulateur Android, ou sur un téléphone branché en USB avec le débogage USB activé, et installe le certificat. Le trafic passe par ADB, donc aucun des deux n'a besoin d'être sur ce réseau.

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = adb est introuvable. Installez Android Studio, ou les outils de plate-forme Android, puis recherchez à nouveau.

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = Aucun appareil n'est connecté. Démarrez un émulateur, ou branchez un téléphone avec le débogage USB activé, puis recherchez à nouveau.

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = Configurez un appareil pour qu'il envoie son trafic ici. Défaire, ou fermer tracewing, retire à nouveau le proxy.

# Undo
t71fd4acf = Défaire

# Android emulators and USB devices
t379e433c = Émulateurs Android et appareils USB

# Find devices
t54d4af39 = Rechercher des appareils

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code, Cursor, VS Code et d'autres outils d'IA peuvent lister et lire les flows capturés par tracewing, via son serveur MCP. Les jetons et les cookies restent exclus sauf si les Paramètres les autorisent. Dans Claude Code :

# Elsewhere, in the tool's MCP settings:
tec38833c = Ailleurs, dans les paramètres MCP de l'outil :

# Flows
t4db77db2 = Flows

# Choose the flow above or below
tecb3adec = Choisir le flow au-dessus ou en dessous

# The first or the last flow
t7e278a00 = Le premier ou le dernier flow

# A page up or down
t23b1814b = Une page vers le haut ou le bas

# Filter
tf4a9c097 = Filtrer

# Pause or resume recording
t6e21519a = Mettre en pause ou reprendre la capture

# Clear the list
tf2a55d1f = Vider la liste

# The chosen flow
te0a729de = Le flow sélectionné

# Its request
tb3970784 = Sa requête

# Its response
t221fc168 = Sa réponse

# Send it again
tef7d1944 = Le renvoyer

# Edit it and send it
t6bc99e5c = Le modifier et l'envoyer

# Copy it as cURL
tc99a11a3 = Le copier en cURL

# Select all of the part shown
tf9168168 = Tout sélectionner dans la partie affichée

# Copy the selected text
t00b090d3 = Copier le texte sélectionné

# The window
t2af18488 = La fenêtre

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = Configurer, Trafic, Règles, En pause, Composer, Sessions, Paramètres

# Show or hide the sidebar
t67adca04 = Afficher ou masquer la barre latérale

# Show or hide the inspector
ta5a7b9e9 = Afficher ou masquer l'inspecteur

# The inspector beside the list or below it
t51e214f7 = L'inspecteur à côté de la liste ou en dessous

# These shortcuts
tbdf3b62b = Ces raccourcis

# Close them
tf3a4b1ef = Les fermer

# Keyboard shortcuts
t31eaddb7 = Raccourcis clavier

# Close
tcd86acc3 = Fermer

# The certificate: {e}
t0235648c = Le certificat : { $e }

# The terminal
tf542b2f4 = Le terminal

# The system proxy: {e}
tc99726ce = Le proxy système : { $e }

# Blocking HTTP/3: {e}
t024ec871 = Blocage de HTTP/3 : { $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:, status:4xx ou texte dans les corps

# A name, as iPhone test
t19f89214 = Un nom, comme iPhone de test

# The search index: {e}
te3c971c9 = L'index de recherche : { $e }

# An earlier session: {name}
t73446a1d = Une session précédente : { $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = Lecture seule. La recherche couvre davantage à mesure que son index se termine. La capture continue.

# Read-only. Recording goes on.
te2f35d9e = Lecture seule. La capture continue.

# Back to live
t317c0453 = Retour au direct

# {shown} of {total}
tf2cc0563 = { $shown } sur { $total }

# Recording
te75cf20c = Capture

# System proxy off · {addr}
t7e813d25 = Proxy système désactivé · { $addr }

# System proxy…
t29e7dcd0 = Proxy système…

# System proxy on · {addr}
t319ad6b9 = Proxy système activé · { $addr }

# Apps
t53d91aad = Apps

# Devices
t27acf320 = Appareils

# Hosts
tcf9aefb4 = Hôtes

# All hosts
tfc7bfe37 = Tous les hôtes

# Hidden
t3c2dfe59 = Masqués

# Newest
t324b6d0b = Plus récents

# No flow fits this filter
t7eb295f4 = Aucun flow ne correspond à ce filtre

# Nothing yet
tbd00747c = Rien pour l'instant

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = Activez le proxy système ci-dessus, ou dirigez une app vers { $addr }. Pour essayer : curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = À côté de la liste

# Below the list
t23658daf = Sous la liste

# Drop it on a place · Esc puts it back
t0fa632ba = Déposez-le sur un emplacement · Échap le remet en place

# Traffic
t4b589266 = Trafic

# Certificate trusted on this computer
t7b93f551 = Certificat approuvé sur cet ordinateur

# Checking the certificate…
tbf8942c3 = Vérification du certificat…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = Installation du certificat ; le système peut vous demander de confirmer…

# Removing the certificate; the system may ask you to confirm…
t913da735 = Retrait du certificat ; le système peut demander une confirmation…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = HTTPS échouera tant que le certificat n'est pas approuvé : configurer

# settings.toml was not used: {problem}
tc3e1f9c8 = settings.toml n'a pas été utilisé : { $problem }

# The proxy stopped: {failure}
t04dcff11 = Le proxy s'est arrêté : { $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = Le disque est presque plein : les corps ne sont pas conservés

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
        [one] { $count } flow non capturé
       *[other] { $count } flows non capturés
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
        [one] { $count } flow sans corps
       *[other] { $count } flows sans corps
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
        [one] { $count } flow le plus ancien supprimé pour rester sous { $size }
       *[other] { $count } flows les plus anciens supprimés pour rester sous { $size }
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
        [one] { $count } flow
       *[other] { $count } flows
    }

# Export as HAR
t95b6b766 = Exporter en HAR

# Shortcuts {keys}
t4ab0ecb4 = Raccourcis { $keys }

# Columns
tea156fdc = Colonnes
