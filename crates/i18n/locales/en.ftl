# tracewing's English, from the code (scripts/i18n.py update writes it:
# do not edit). The source for translations.

t731220a4 = Free software
t4e98bf3c = Under the GNU General Public License, version 3 or later
tfc21d38b = View license
t10c7007a = Made by HivePaaS
t49e74c86 = Built in Rust, on GPUI
tf6a49f4d = Source code
t610dfb6f = Something wrong?
ta3e29034 = Tell us, with the version above
t7b1728e7 = Report a problem
te8345543 = What changed
t8e6f2052 = Every release, and what is new in it
tc52ab62f = Release notes
t14b291a4 = © { $year } HivePaaS. The tracewing name and logo are trademarks of HivePaaS.
t8dc21305 = Copied
t3d37483c = Copy version info
t6bead5ae = Version { $version } · { $os } { $arch }
t86e5ad3e = A native HTTP(S) debugging proxy that keeps up with big sessions.
tb25cf729 = More from HivePaaS
tb730ab28 = Free and open source, like tracewing.
tc9d13419 = A self-hosted platform for your apps, built on Docker Swarm: an open-source alternative to Heroku, Render and Coolify.
t6a770b0a = 300+ one-click apps
tb0bfc3e9 = Deploy on push
t4e4180fc = Pull request previews
t9e9776ed = Automatic TLS
tebe3a172 = Live logs and metrics
t0274f036 = Apache-2.0
t0632975a = Visit hivepaas.com
t0f4c758a = A fast, native SSH client for Linux, macOS and Windows.
t0f8af34d = SSH and SFTP
tc1c4446d = Follows your ssh_config
tae9bcfa8 = A real terminal
t8622a5c1 = GPL-3.0-or-later
t1d2ed540 = Visit shellwing.app
t5d2a9fa0 = A file archiver that puts every core to work and opens archives safely.
t4d2518b8 = ZIP, 7z, RAR and tar
t553f5ef1 = Every core at work
t3ffc07c8 = Safe by default
t61def008 = Browse S3, WebDAV and SFTP
t11cbc28f = Visit packwing.app
t2ba67348 =
    { $n ->
        [one] … { $count } line { $closer }
       *[other] … { $count } lines { $closer }
    }
ta8dc6eab = … { $size } more
t0a4220dc =
    { $n ->
        [one] { $size } { $format } · { $count } line of JSON
       *[other] { $size } { $format } · { $count } lines of JSON
    }
t16aa1c6d = The first { $count } rows are shown.
tbdecf3c8 = Copy row
te26431fe = Copy column
t9bc3ec63 = Open this line
tdb2d6aa8 = Forget how { $field } is read
tc75725b1 = Always read { $field } this way
tdd38ff77 = Clear what was read
t3849d44e = Unfold all
t51332550 =
    { $n ->
        [one] { $size } · { $count } line
       *[other] { $size } · { $count } lines
    }
t49fe2cb3 = { $size } · reading { $percent }%
t1d03e346 = { $call } could not be put in the body as calldata. Write it as hex, or let tracewing write the body.
tc159f8f5 = That is not a call written out.
t61568d43 = No call named { $name } is known here. Write its types with it, as transfer(address,uint256)(…), or add its ABI under Contract ABIs in Settings.
t48b75c8d = { $name } is more than one call: { $found }. Write the types with it, as { $first }(…).
t544218b9 = { $types } are not types a call can be written with.
t14015b69 = The values do not fit { $signature }: { $why }
t332cfa6d = Appearance: { $now }. Click for { $next }.
tdc6588ac = api.example.com, or *.example.com
t73c66160 = Choose
t6c7ace6e = Choose the certificate's file.
taf39b2e1 = Say which host it is for.
t3a0298f3 = Not used: { $why }
t21a5901d = Remove
te3ce8812 = Client certificates
tcd3f6864 = For servers that ask the client for a certificate (mutual TLS): tracewing shows the one given for their host, in the app's place. A PEM certificate and its key without a passphrase; the files stay where they are. Each applies to the next connection.
t786bc275 = None yet.
t164f4d6c = Certificate…
t23b63bc7 = none chosen
t7aa250a8 = Key…
t13bc7052 = in the certificate's file
t9dc3aa14 = Add
t98a2c750 = Method
t005ef20f = Status
t81f51acc = App
t302af18f = Host
teb66e456 = Path
t218e02a6 = Operation
ta6478e7c = Size
tdfe4e404 = Time
t61e9eac0 = Number
td2c32731 = The request is in { $path }. Give it to your IT team to sign as an intermediate CA, then take in what they send back.
t3118cd2d = Take in
tb68eb888 = Signed by { $issuer }: restart tracewing to use it. Nothing needs installing on this company's machines.
tb4233c11 = It cannot be used: { $why }
t4a1a1e86 = Back to tracewing's own certificate: restart tracewing to use it.
tcb26a623 = Now: signed by { $issuer }, until { $until }.
t900843cd = Now: tracewing's own, which signs itself, until { $until }.
t1df2d89b = Certificate signed by your company
t0489607f = Where IT lets no new root certificate in: tracewing makes a request for its CA's key, your IT team signs it as an intermediate CA under the company's root, and tracewing uses that instead. The company's computers trust it already, with nothing to install.
t61f8c4ae = Make the request (CSR)…
t2f79e9d7 = Take in the signed certificate…
t9c075d1b = Back to tracewing's own
t9d6abb2a = A body was not kept, so the two cannot be compared.
t6a8fff7f =
    { $n ->
        [one] { $count } field differs
       *[other] { $count } fields differ
    }
tcc37e7ea =
    { $n ->
        [one] … { $count } line the same
       *[other] … { $count } lines the same
    }
t9f32d942 = Nothing to compare.
t7569633e = URL
t4a57ebd5 = Protocol
td155d06d = Type
td1a4a19d = Sizes
t428a1095 = Body
t334f5989 = Headers
t9e89e783 = Query
t82454152 = Request
t55a982fe = Response
tb43ef094 = Neither has headers kept.
t7c44fa89 = Trailers
t4d8203e0 = Neither URL has a query.
ta8565d31 = Comparing…
tfa4c2902 = No body
t1d894c39 = { $kind }, its first { $size } compared
tf2c47725 = Neither has a body.
t85152e01 = The bodies are the same.
t0f651f77 =
    { $n ->
        [one] { $count } field differs.
       *[other] { $count } fields differ.
    }
taae62a3e = The first { $count } are listed.
tf57a3157 =
    { $n ->
        [one] { $count } line differs.
       *[other] { $count } lines differ.
    }
t3308bd9c = Fields
t8a1a00dc = Lines
td57a0a54 = Compare
t71ce2de9 = #{ $a } beside #{ $b }: what only #{ $a } has is red, what only #{ $b } has is green.
te519d36e = Swap
t644042d2 = Back to Traffic
td3a1f5f7 = As agreed
tacdcf5cb = "{ $method }" is not a method
t1f9e5368 = its request body was not kept whole, so it cannot be sent again
t4df37bea = name: value, one a line
t550b0a98 = No flow is chosen
t296762f8 = This flow: { $e }
t3220a10a = The body is not a { $message }: { $e }
t781059cd = The body of { $bytes } bytes was too large to keep when the window last closed, so this request came back without it.
t5e2cb6ff = This request came back without its body: the body it had was bytes that are not the text shown for them, and writing the text down would have sent something else.
t83b6cd6e = Protobuf { $message }, written as JSON: it is sent as Protobuf.
tcaecdf04 = Written out in the body and sent as calldata: { $calls }
te85cbcea =
    { $n ->
        [one] { $names } is the client's to work out, so what was written for it did not go.
       *[other] { $names } are the client's to work out, so what was written for them did not go.
    }
t4916bc86 = { $version } is how tracewing is asked. What the server is asked with is settled between tracewing and the server, as it is for every other flow.
t5a78825c = Stopped: no answer was read. What reached the server shows in Traffic.
tb5209e72 = Ask with
te946e7bf = Sending…
tf28e14cf = Send
t4b7f7705 = Stop
t8b242db1 = New
tf43ea06c = … { $count } more bytes: the flow in Traffic has them all.
taa3cc3ff = Body · { $message }
t18049f93 = Compose
t7a9cf113 = A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t961a5ea0 = { $version } both sides
t3d237646 = { $client } to the proxy, { $server } to the server
t9e437c74 = Queued
tdb89a899 = Connect
tc9450e48 = Waiting
t6964bb19 = Download
t6935fbd3 = No file picker: { $e }
t725a0e89 = Writing the HAR…
t643f96ee =
    { $n ->
        [one] { $n } flow written to { $path }
       *[other] { $n } flows written to { $path }
    }
tef8ec569 = The HAR was not written: { $e }
t538b10e9 = Open
tc0e7c8f4 = Reading { $path }…
td848c31b =
    { $n ->
        [one] , { $n } entry left out
       *[other] , { $n } entries left out
    }
t610d7810 =
    { $n ->
        [one] { $n } flow read from a { $format }
       *[other] { $n } flows read from a { $format }
    }
t7f49510f = The file was not read: { $e }
ta7b4c380 = it is no longer kept
tc6932467 = { $size } saved to { $path }
t189900b4 = The body was not saved: { $e }
t9f5c89a3 = Fiddler session
tb4f56f2b = Charles session
t75afe364 = All
te9ed60a6 = Errors
td10fcaa3 = Media
t3e1cd17a = Find
t66d1107a = Not found
t284579ba = Looking…
tb689f139 = { $now } of { $count }
t2034fd8e = Match case
t1d601c34 = Whole word
t3f9a29fb = Regular expression
t956e28b3 = The one before (Shift+Enter)
t0c8af534 = The next (Enter)
t653214db = Close (Esc)
teca2074f = Rule: { $rule }
t003420a4 = Events
te8ce2db0 = JSON lines
t8090a1ba = Not decrypted
td35ec4c9 = open
t51c96267 = Reading the request…
tba9e5628 = The request's body was not kept, so its query cannot be shown.
t38c4d30b = An operation without a name
t53045460 = { $n } of { $count }: { $name }
tf23e3c2a = A persisted query: the server keeps its text, and the request sends only its hash.
t7996f062 = Variables
t58e25863 = Cut
t658f3664 = Copy
tb34a0288 = Paste
t6af21072 = Select all
t3343999e = This flow is gone.
tf85fe84f = This host is not decrypted, so what it sends cannot be seen.
t42be95fb = Still coming: it shows once it ends.
t31e4d044 = No body.
t884aad28 = This body was not kept: the disk was nearly full, or too much was waiting for it.
te46616b5 = This body was not kept.
tf9a7ab60 = This body is gone: it was dropped with the oldest flows, or cannot be read.
t620a039b = Cannot open this body: { $e }
t4def410e = Inspector
tb6914c4d = Narrower
t397967e6 = Shorter
t4b19aae8 = Wider
t245ae741 = Taller
tc7be91e8 = Choose a flow to see it here
ta05ec37a = Not decrypted. New connections to this host go through unread.
t3cbba118 = If an app fails with this host, it may pin its certificates.
t0ac0e062 = Make a rule
tb7367e2c = Decrypt this host
t5fa113f6 = Do not decrypt this host
t04e060e6 = Request and response
t70397119 = Request and response one at a time
t3d35ce69 = Request and response side by side
t83c63123 = Request above the response
td70ceb7f = Table
t7e09470d = Pretty
ta689aa91 = Raw
te97ed633 = Picture · { $size }
te61b841a = Messages { $count }
t04b0146d = Headers { $headers }
td58eb84b = Cookies { $cookies }
t4756f9f5 = Query { $query }
tfa6e7203 = Form { $form }
t09ea4ff9 = Connection
t1a435961 = Find (Ctrl+F)
te2c44424 = Save body
t4f34d900 = More
tf5910557 = Send again
tf837e542 = Edit and send
t866d1e3e = Copy as cURL
t59253578 = Click another flow…
t58dfe040 = Compare with…
tef15286c = Compare with the call before
t9c1ad448 = Copy as…
t1e92fb27 = This picture cannot be drawn. Save the body to look at it.
tc40f2321 = Opening…
t8a86fa50 = this value
t17d1ec47 = The whole line
t1942c7ae = line { $number }
t4df6b638 = Readings
t60beaefd = Close the readings
t6af47169 = a guess
t8c0f1d8b = Read as
te030653c = choose…
t6d510d3d = Read from { $from }
t3506a32c = Back to the body
t8c2c37b1 = No response
tce693254 = Tunnel, not decrypted
t2d65757d = GraphQL: { $operation }
tf8162891 = RPC: { $method }
tb12d299a = Where the time went
t84144a81 = Over connection { $id }, made before: no time went on connecting.
t627147b4 = { $app }, pid { $pid }
t0efce58b = Not found: it may have ended, or belong to another user
t2b964a37 = On another device
t6d3cd99e = CLIENT
tbdcbb413 = Device
tc4253b6b = Program
t700560f3 = Address
te3b27612 = SERVER
t1ef5a00a = What this connection was is no longer kept: it went with older flows.
t0427c9b1 = Not decrypted: tracewing passed it on without looking.
tf348eb92 = No connection was made.
tac71808e = None: it was answered without one.
t0ec66a8c = { $id }, given up during TLS
t7fe32cfc = { $id }, made for this flow
ta904c078 = { $id }, made before and used again
t6d7302f2 = Server
t05606349 = None: plain HTTP
te476fbb8 = THE SERVER SHOWED NO CERTIFICATES
ta51f3107 = WHAT THE SERVER SHOWED, ITS OWN FIRST
te6f31dce = NO CERTIFICATES
t1c902790 = CERTIFICATE
td5295f62 =
    { $n ->
        [one] { $n } CERTIFICATE, THE SERVER'S FIRST
       *[other] { $n } CERTIFICATES, THE SERVER'S FIRST
    }
t13ced472 = tracewing would not go on with this server: { $error }. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t5b3bed6a = Cipher
tada7afdb = none
t0a14b2d8 = not valid until { $when }
t932538c2 = expired { $when }
t679bcb51 = expires { $when }
t4cd375ad =
    { $n ->
        [one] { $names }, and { $n } more
       *[other] { $names }, and { $n } more
    }
t15a1ce5a = Issued by { $issuer }
t0e47d4f2 = Valid { $from } to { $to } · { $when }
tda75e459 = Names { $names }
td1f0d98b = Serial { $serial }
tb8739fec = These headers were not kept.
tf3dacb5e = No headers.
t3447e9de = Trailers, sent after the body
t6dbd239f = File { $filename } · { $kind } · { $size }
t902a6ece = no type
t58e85ab2 = No cookies were sent.
tf57dbabc = No cookies were set.
t2db33271 = JWT · { $algorithm } · signature not checked
t05f8b533 = JWT · signature not checked
t114f9980 = Header
ta0639745 = Payload
t7f8fa9c0 = The simulator did not take it: { $why }
t5561fd0b = General
t6974ad65 = Network
tb88e4cd9 = Certificates
t5f8586f8 = Decoding
t34eddf06 = Storage
t294d7ea4 = AI tools
t58e096de = About
t8ad2f8cf = Light
t45570065 = Dark
t8f3184bc = System
t76711a1a =
    { $n ->
        [one] streamed in { $count } chunk
       *[other] streamed in { $count } chunks
    }
tfa48fb30 = stopped: { $stop }
tc8bbc724 = { $input } in · { $output } out tokens
t33b52a3c = Reading the call…
tf94ea0ff = Answer
t737ece33 = Thinking: { $thinking }
t017882e4 = Calls { $name }{ $id }
td9b71d16 = Nothing yet.
t6a16c074 =
    { $n ->
        [one] { $text }… and { $count } more character
       *[other] { $text }… and { $count } more characters
    }
t69975ed7 = Copy the answer
t4e3af9c3 = Asked
tb871bcf4 = Tools offered: { $tools }
tba30b5fa = Not kept
tb888a3c3 = This message was not kept: too much was waiting, or the disk was nearly full.
tdb0ec01f = This message is empty.
t51e7624c = No messages yet
t3558745a = { $said } kept, of { $size }
t351a1976 = Close this message
t8d883a0f = The socket has closed.
tbea55dfd = Too many are still waiting to go. Try again.
te1ef193a = To the server
t1bcabce4 = To the app
t17809004 = Layouts
t2b6aeb69 = Keep the panels as they are now under a name, to come back to them.
t555cbe68 = Keep
t2a8515ce = { $what }: { $text } is not a whole number
t23a45ef9 = Latency
ta4995035 = Down
t42f35940 = Up
t69db303a = Loss
t7cfd6404 = Give a latency, a speed or a loss to slow it down by
tefe18ab9 = Latency, ms
t08c4ae33 = Down, kbit/s
td55cfe9a = Up, kbit/s
tea4af439 = Loss, %
t77a1f991 = Only these hosts, like *.example.com; all if empty
tdce4d67c = Not kept for next time: { $e }
td51ddfce = Slow network
tf0f34db0 = Off: traffic goes as fast as it can
t9525b78e = Custom
tfe16992b = Only these hosts
tca77bbdb = Apply
t5ee4034d = Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t891b076d = Header line { $line } has no ":"
te3247b96 = Header line { $line } has no name
td6aec821 =
    { $n ->
        [one] { $n } bytes that are not text: they go on as they came.
       *[other] { $n } bytes that are not text: they go on as they came.
    }
t8cf68b4c = Nothing is paused
t40a529af = "{ $status }" is not a status
t1f3f6dd2 = REQUEST
tc85f77de = RESPONSE
t270a6209 = MESSAGE TO THE SERVER
t7d928b05 = MESSAGE TO THE APP
tf9ab4327 = Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
t1204e859 = { $kind } on { $url }
t093a86f6 = for { $method } { $url }
tc840fed6 = The request, before the server gets it
t6683d72a = The response, before the app gets it
tda2ebc5d = A socket's message, before the server gets it
tbce07e9d = A socket's message, before the app gets it
tae0ed984 = Message
tb593e730 = Send as it came
t108524ac = Drop
tde281feb = Paused
tea613769 = Nothing is waiting.
t745634c6 =
    { $n ->
        [one] { $n } waits to be sent on.
       *[other] { $n } wait to be sent on.
    }
tdce25640 = Send all as they came
tfceba380 = The list was not saved: { $e }
tc1b8764c =
    { $n ->
        [one] { $count } service: { $names }
       *[other] { $count } services: { $names }
    }
t14f9744c = https://api.example.com or http://localhost:50051
t6aad8485 = Reading…
tde674bd8 =
    { $n ->
        [one] { $count } file
       *[other] { $count } files
    }
t8a1cd2f6 =
    { $n ->
        [one] { $count } message
       *[other] { $count } messages
    }
t51f71902 =
    { $n ->
        [one] { $count } service
       *[other] { $count } services
    }
t591ce5ea =
    { $n ->
        [one] and { $count } more problem
       *[other] and { $count } more problems
    }
ta4401d1a = Protobuf schemas
te5f4f07c = Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.
tb5930e43 = Add a folder…
ted08dbe5 = Add files…
t2f0dcc5d = Read again
tf6948a5a = Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.
tf9cf22fe = Asking…
t533a3183 = Fetch
te6f78626 = Find a message
t87b6a380 = Read as { $message }
t1b90af95 = Read as…
t8826d766 = Requests to { $path } on { $host } are read as the message chosen.
tbbe012da = Answers from { $path } on { $host } are read as the message chosen.
t134c9a06 = Automatic (gRPC path or messageType)
t70ff68f9 =
    { $n ->
        [one] and { $count } more: type to narrow them
       *[other] and { $count } more: type to narrow them
    }
td38c5cc8 = Copy reading
t92a8694c = Nothing to read in this
t4f68f91a = Write the server's address first
t8c4d03a6 = "{ $address }" is not a server's address
tfa5b7ad8 = The server has no services to describe
tdca19bdc = The server sent no files for its services
t345fdb40 = The server answered { $status }: it may not offer gRPC reflection
tb629329e = The server answered { $code }: it may not offer gRPC reflection. { $message }
t6bbb6207 = The server answered nothing: it may not offer gRPC reflection
t3a48373c = The password is kept in the system's keychain.
tf84cb18b = The password is kept in relay.toml, which only you can read.
t5a08b6d0 = The password is kept in relay.toml, which only you can read: the keychain cannot be used ({ $why }).
tb5f1ef96 = New connections go through the { $kind } proxy { $address }.
t4a64a08f = Connections go straight to servers.
tbc6a2da3 = relay.toml cannot be used, so connections go straight: { $problem }
tb945a053 = http://proxy.example.com:8080 or socks5://proxy.example.com:1080
tc67dfdb1 = Go through another proxy
tf46c980f = Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080, or http://DOMAIN%5Cname:password@proxy:8080 for a Windows domain; the password is then kept in the system's keychain, not shown again. Basic or NTLM, as the proxy asks. Empty, tracewing goes straight to servers.
tcbc7d298 = Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host.
td717c770 = Sent by the server, not asked for
t2c29f04d = any
t5579c3d4 = Failed
ta74e5f73 = The server answered 200: a call that fails says so in the body.
t534e422e =
    { $n ->
        [one] { $count } item
       *[other] { $count } items
    }
t0039810a =
    { $n ->
        [one] { $count } field
       *[other] { $count } fields
    }
tbdb1ff6a = and { $count } more
ta7ee514c =
    { $n ->
        [one] { $count } byte that does not read as one
       *[other] { $count } bytes that do not read as one
    }
t1ce6ad3b = { $program }, { $count } bytes
t6b166ff2 = succeeded
t46ea918c = failed: it was mined, and what it asked for did not happen
t188b58f6 = none: it makes a contract
t36ffce30 =
    { $n ->
        [one] { $count } byte, which the signature does not say how to read
       *[other] { $count } bytes, which the signature does not say how to read
    }
t0965779e =
    { $n ->
        [one] { $count } place
       *[other] { $count } places
    }
t95369f92 =
    { $n ->
        [one] { $count } blob
       *[other] { $count } blobs
    }
tc2fde48e = 0, the kind from before EIP-2718
teaf455ca = The bodies were not kept, so what the call sent cannot be shown.
tf0ac7103 = and { $count } more calls
t9ffc3c7c =
    { $n ->
        [one] { $count } function
       *[other] { $count } functions
    }
t4dd03992 =
    { $n ->
        [one] { $count } event
       *[other] { $count } events
    }
t4fba4c4e =
    { $n ->
        [one] { $count } error
       *[other] { $count } errors
    }
t08fb00f2 = Contract ABIs
tfadffbd2 = Folders or files of ABI JSON, as solc, Hardhat and Foundry write it, to name the functions, events and errors in a JSON-RPC call instead of showing four bytes. They are read again when they change. Nothing is sent anywhere: a signature is worked out from the ABI on this machine.
tbd074002 = Block
td8edab50 = Answer from a file
te3cd85d7 = Send to another host
tfad4aaaa = Set a request header
tcbbc20d0 = Set a response header
t5228dd84 = Remove a request header
teeb5735a = Remove a response header
t73d9419b = Set a query parameter
t12633da9 = Remove a query parameter
t41101201 = Change the request body
tdd259b73 = Change the response body
t4bbdfba8 = Change the status
t40ecc39b = Pause the request
t18a34799 = Pause the response
te5d065fe = Pause a socket's messages to the server
t482173e8 = Pause a socket's messages to the app
t65a52970 = Run a script
t86d04e51 = Ask the server with HTTP/1.1
tf90c44e9 = Ask the server with HTTP/2
t8f3ebb6c = Status to answer with, like 403
t9fb9ae4c = Full path of a file or folder
t01bacb53 = Full path of a .js file with onRequest or onResponse
tdfedd004 = Where to, like http://localhost:3000
t80793085 = Header's name
t43968778 = Its value
t313c8e35 = Parameter's name
t8255fc87 = Text to find, or empty for the whole body
t03671fd5 = What to put in its place
t5e4d3088 = Status the client gets, like 500
td43128d3 = "{ $first }" is not a status
t1727e984 = Block with { $status }
taefc08fa = Answer from { $path }
t5e256873 = Send to { $to }
taa230766 = Request: { $header }: { $value }
t30f9e2c2 = Response: { $header }: { $value }
t6f0b3f5a = Request: no { $header }
t23960a2e = Response: no { $header }
t0ee6c1c0 = Query: { $name }={ $value }
tf2e7bf27 = Query: no { $name }
tad048e53 = Status { $status }
t2f9fde3f = Run { $path }
t0a50b346 = Ask the server with { $version }
te4856c36 = Request body: { $replace }
t8600dc42 = Response body: { $replace }
taec471a9 = Request body: { $find } → { $replace }
tc1449525 = Response body: { $find } → { $replace }
t0fe07306 = Name
t4dd91658 = URL, like api.myapp.dev/v1/*
tb2b869ac = Method, or empty for all
t4a2e843a =
    { $n ->
        [one] { $n } rule written to { $path }
       *[other] { $n } rules written to { $path }
    }
te7ac257f = The rules were not written: { $e }
t573e0af4 = Import
t32a3ce94 = { $path } could not be read: { $e }
tb9f7257a = Nothing was imported: { $e }
tfce667e2 = Failed: { $why }
t3e82a85a =
    { $n ->
        [one] Ran once. Last said: { $last }
       *[other] Ran { $n } times. Last said: { $last }
    }
t47b9c718 =
    { $n ->
        [one] Ran once.
       *[other] Ran { $n } times.
    }
tc2c76cb1 = Edit
t5797ea6a = Delete
t9160f393 = Change this rule
t55a722a0 = A new rule
tca566a57 = It waits under Paused until you send it on, changed or not.
t4d2d5d68 = Save
t35afca3b = Cancel
t428b2d80 = Rules
tbe44b31e = They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
t5edc205d = Import rules…
t6e09ca20 = Export rules…
tbc5e0866 =
    { $n ->
        [one] { $n } rule added
       *[other] { $n } rules added
    }
t65dc1281 =
    { $n ->
        [one] { $n } here already
       *[other] { $n } here already
    }
ta81e95e2 = left out: { $why }
t6aef5c37 = Copy all
t2fe8ff9f = Too large to copy all
t6dc463eb = No flows
t52c1a402 = Name it to keep it
t6bfd918c = Nothing wrong with it.
tf756539b =
    { $n ->
        [one] SQLite found { $count } problem: { $first }
       *[other] SQLite found { $count } problems, the first: { $first }
    }
t40214658 =
    { $n ->
        [one] { $count } flow rescued into a new session. The end of the damaged one could not be read, so there may be more.
       *[other] { $count } flows rescued into a new session. The end of the damaged one could not be read, so there may be more.
    }
t26f5c0cc =
    { $n ->
        [one] { $count } flow rescued into a new session; { $lost } could not be read.
       *[other] { $count } flows rescued into a new session; { $lost } could not be read.
    }
t878e5c80 = { $size } · written by another version, so it cannot be opened
tf94fb7c2 = { $size } · damaged, so it cannot be opened
t7dfb1f1b = { $size } · cannot be read: { $why }
t3f3c0eb4 =
    { $n ->
        [one] { $count } flow · { $size }
       *[other] { $count } flows · { $size }
    }
t3cb3e6fc = Recording now
t4d6458cc = Export HAR
t1eea47d9 = Rescuing…
te43c8d2b = Rescue what reads
t87f62cb7 = Checking…
t592c8de7 = Check
ta7737f13 = Delete for good?
tb69a8aec = Sessions
t6391f080 = A session with a name is kept until you delete it. Of the others, the newest { $kept } stay.
td5507854 = Working…
tae8622fc = Open a session file
td8955e6d = HAR, Fiddler .saz, or Charles .chlsj
t17bccaf2 = Start a new session
t3435c213 = Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t9e7d8362 = Other devices
t13e626b3 = Let other devices use the proxy
t76098911 = Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
tcd0cf113 = Servers
tfea2d91b = Trust what this computer trusts
t1705bfb4 = Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t53722007 = Reading values
t3f1c9b5a = Underline what can be read
t9e1ebac7 = A value with a reading behind it — a time, a token, something packed — is underlined, so that a right click on it is worth trying. Off, the readings are still there; nothing says so.
te1d26d3c = Let AI tools see tokens and cookies
tb496980f = AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
t663de1ba = Let AI tools add rules and send requests
t46d53486 = Through the MCP server, an AI tool may then add and remove rules, which change traffic as yours do, and send requests through tracewing, again or changed, which reach their servers. Off, it can only read. Applies at once.
tfec0f477 = HTTP/3
te42733a9 = Keep apps off HTTP/3
t2ff32f01 = HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
ta20fa102 = Size of a session
td2914f46 = Past this, the oldest flows are dropped.
t8674a3b3 = Room to leave on the disk
t5b3a31a3 = With less than this free, flows are still listed but bodies are not kept.
td356194a = Earlier sessions to keep
t66556513 = Older ones are deleted when a new session starts.
tdb4d0409 = Bodies
t777508dd = Most kept of one body
t386fee28 = It still reaches the client whole.
tc3fe2efd = Most unpacked for viewing
tf9c49515 = Stops a small download that unpacks to many GB.
ta8996af1 = Small body size
t20c108b4 = Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
tee503c2e = Compress plain text on disk
t6d70f53f = Takes about half the room, and is skipped when traffic is too fast for it.
td386c9ef = Compression level
td42a987d = 1 to 19. Higher is smaller and slower.
tc646a2c9 = Search
t0c3e0b3d = Search URLs
t3f22d666 = Finds any part of a URL as you type. Costs little.
t8eec0293 = Search bodies
tcac3872a = Finds words in bodies. Can take as much disk as the bodies do.
tab293b21 = Searched part of each body
t0bf65f4a = Words past this are not found. More takes more disk.
ta99888cb = Under heavy traffic
t7d6ff799 = Memory for bodies waiting to be written
t5b268fb2 = Past this, flows are listed without their bodies. Traffic is never held back.
t07b9de08 = Memory for large bodies waiting to be written
tfe61a4f3 = Past this, large bodies stop being kept.
ta8606963 = { $key }: "{ $text }" is not a whole number
t9c4debaa = { $key }: must not be negative
td3f56fa8 = { $name } (default)
tb92a5ad0 = System ({ $name })
t68a32cf9 = Fonts
t7d000dd0 = Interface
t795db914 = Code
t014e83f3 = For code, the fonts on this computer whose letters are all as wide. Applies at once.
t5606e442 = { $size } (default)
t2c041143 = Text size
tb8052ae6 = In px, for the text of lists and bodies. Everything grows with it. Applies at once.
t31018b9f = Appearance
tb457f0b3 = Hosts not to decrypt
t38e21213 = One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t4b058728 = Settings
t58dcea2d = After a restart
td0200095 = Every setting here is back to its default.
t69f2bb4d =
    { $n ->
        [one] { $n } change takes effect when tracewing starts again.
       *[other] { $n } changes take effect when tracewing starts again.
    }
t71fd4acf = Undo
ta3162a13 = Back to defaults
tb6e892b1 = Fields always read a certain way
t176e5171 = A value read in the inspector can be kept that way for its field, and every later flow from that host reads itself. They are kept in readings.toml in the data folder.
t9e58a80c = Nothing is remembered yet.
tb54bdd97 = { $reading } · every host
t58858d86 = { $reading } · { $host }
t6e3fab8c = Forget
t0a308a8a = Authorities trusted for servers
t571051f9 = tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in { $folder }. They are read when tracewing starts.
t4a567d82 = Left out: { $why }
t65f2d800 = The folder has changed since tracewing started: restart it to use what it holds now.
tc7d7e212 = Open the folder
td72efd5e = Read it again
tcb4394af = TLS keys for Wireshark
t2f92ac10 = Writes the secrets of each HTTPS connection, from apps to tracewing and from it to servers, to a file Wireshark reads: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Anyone with the file can read the traffic it covers. Off again when tracewing starts, unless SSLKEYLOGFILE names a file.
t956ecc4b = Cannot write to it: { $why }
t095d064e = Copy the path
t9a73db9b = Language
tb5c7a1d4 = System ({ $system_name })
tefb7a0f9 = Applies at once.
tb5a48dd2 = Your browser
t62a47808 = Its private key is in the system keychain.
td9c6165d = Its private key is in ca.key, which only you can read.
t453607d0 = Its private key is in ca.key, which only you can read, as the system keychain cannot take it: { $why }.
t2e93b53b = Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t82bd2456 = { $name } goes through tracewing and trusts it.
tfb6feb04 = { $name } goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tc18aec73 = A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
t2188b3f4 = this computer's address, port { $port }
tc8ccf306 = On the device, set the Wi-Fi proxy to { $addr }. Then open http://{ $host } there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
t50ee7452 = Looking for it…
ta41c38ec = Installing. The system may ask you to confirm.
t62b22726 = Removing. The system may ask you to confirm.
t27d6bacc = This computer trusts it.
tb5bcc244 = Installed in: { $places }.
t327bf8fc = Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
tafd8dae2 = tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
t26618699 = Trust the certificate
tb50cbd40 = Install
t0db8faea = Also for command-line tools
t3c9fc304 = Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t76cf4bfa = Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
td2606dbb = Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
tb13a87e6 = This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t83387edf = Apps that use the system's setting now go through { $addr }. It goes back to what it was when tracewing closes, or if it crashes.
tdcc53241 = Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at { $addr } yourself.
t52b88441 = Turn on the system proxy
tcd03e04b = Turn off
t65a0c5a7 = Turn on
tbed6e83a = Check it in your browser
te4e36f65 = Open the test page
t43c06abc = Set up
tf3a7f60b = Three steps to see HTTPS traffic from this computer.
t980eee79 = See traffic
tddf6836c = the browsers' own stores
tc5bba858 = Trusts the certificate.
tf2a0b8c5 = Does not trust the certificate yet.
t00e4c88d = Has no profile yet. Open it once, then look again.
t2a9826b9 =
    { $n ->
        [one] { $holding } of its { $n } profile trusts the certificate.
       *[other] { $holding } of its { $n } profiles trust the certificate.
    }
t67f9cb49 = Has not opened the test page yet.
t99f8c07e = Opened the test page through tracewing, over HTTPS.
t7d4b60bb = Opened the test page through tracewing, but without HTTPS.
t36665e5c = Sends localhost and 127.0.0.1 straight to the server, as all of macOS's own networking does: a name under .localhost, such as playground.localhost, comes through tracewing.
t67bac94a = Goes only where the system proxy sends it: nothing of it comes through tracewing until step 2 is on.
td580efd4 = Browsers
tce72f1ec = Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
t98416222 = Look again
t0e46bb34 = No browser found on this computer.
t0da7783c = Open on its own
t3c727cd5 = UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t64ab600f = Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t2cfe5270 = Apps on HTTP/3
t318358d8 = Stop blocking
t88231e40 = Block HTTP/3
t91c63f1c = Or one app at a time
tc21013a9 = A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
t4b629c72 = Open a terminal
ta550cc97 = iPhone and iPad
ta04ee397 = Flutter apps
t996be047 = 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server { $server }, port { $port }.
t4fa14138 = 2. Scan the code with the camera, or open { $page } in Safari, and get the certificate. Allow the download.
t7127295f = 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t1b22723c = 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t4adfec01 = 5. Open https://{ $host } in Safari. If it says it works, apps that use the system's settings go through tracewing.
t62466f08 = When you are done
t21a2fdae = 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t2090b614 = 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t4f837b10 = 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host { $host }, port { $port }.
t654a829b = 2. Scan the code, or open { $page } in the browser, and get the certificate.
t8263ba6b = 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
tb7fa0462 = 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
t02ad844b = and point to it from AndroidManifest.xml:
t02e57826 = Other apps keep refusing the certificate: their traffic shows as failed flows.
t76754f49 = 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
tf08245e1 = 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
t4ffc0abf = Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t991340f1 = badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t12aba34f = Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t1336b4d6 = Open settings
t2e3c591c = Phones and other devices
td0629ca6 = A phone, a tablet or another computer on this network can go through tracewing at { $proxy }. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
t77fb5f02 = Other devices cannot reach tracewing at one of this computer's addresses: { $problem }
t864ac93f =
    { $n ->
        [one] { $n } device went through tracewing ({ $names }). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes.
       *[other] { $n } devices went through tracewing ({ $names }). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
    }
t84db6961 = Set up. The certificate is among the system's until the device restarts.
tbcad7fa7 = Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tfd0c5617 = Apps that trust only the system's certificates will still refuse it. On a device adb can root, as an emulator with a Google APIs image, tracewing puts it among the system's instead.
t14b6b86d = An emulator.
t67dafc14 = On USB.
tff6a046e = Accept the USB debugging prompt on the device, then look again.
t13f68b64 = Still starting. Look again in a moment.
t2b7d71da = adb says it is { $state }.
tc28e5d10 = Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
t4af01006 = adb was not found. Install Android Studio, or Android's platform tools, then look again.
tc65b7cf7 = No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
t9144d1fc = Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t379e433c = Android emulators and USB devices
t54d4af39 = Find devices
t01863e29 = Trusts the certificate in the iOS simulators running on this Mac. They go through this Mac's proxy, so that is all they need.
t2bd95662 = xcrun was not found. Install Xcode, or its command line tools, then look again.
tbd7bf840 = No simulator is running. Start one from Xcode, then look again.
tc88c5c9b = Trust the certificate in a simulator. It stays until the simulator is erased; an app already open may need opening again.
tdbe3bd38 = The certificate is trusted.
ta1c67066 = Running.
tb198618d = iOS simulators
ta969b661 = Find simulators
tf4b6cd8a = The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
t2dc39047 = Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
tec38833c = Elsewhere, in the tool's MCP settings:
t4db77db2 = Flows
tecb3adec = Choose the flow above or below
t7e278a00 = The first or the last flow
t23b1814b = A page up or down
t05e6ebf7 = Filter the list; after a press in the inspector, find in it
t6e21519a = Pause or resume recording
tf2a55d1f = Clear the list
te42cd067 = Compare the flow clicked with the chosen one
te0a729de = The chosen flow
tb3970784 = Its request
t221fc168 = Its response
tef7d1944 = Send it again
t6bc99e5c = Edit it and send it
tc99a11a3 = Copy it as cURL
tdf4221d9 = The part before or after the one shown
t20d9b7c0 = Save the body shown
tf9168168 = Select all of the part shown
t00b090d3 = Copy the selected text
t803a686e = Read the selected value the next way, and round again
t8c35fa5a = Send it on, as edited
t16f7abb5 = Send it on as it was
t4e7f48b9 = Drop it
t2af18488 = The window
t9df89562 = Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
tad71f882 = Back or forward, as the mouse's own buttons do
t67adca04 = Show or hide the sidebar
ta5a7b9e9 = Show or hide the inspector
t51e214f7 = The inspector beside the list or below it
ta4b4bd5e = The next or the previous field
tbdf3b62b = These shortcuts
tf3a4b1ef = Close them
t31eaddb7 = Keyboard shortcuts
tcd86acc3 = Close
td0d9c66b = The signature in this body now works out to { $now }, not { $was }: it is no longer the transaction that was signed. tracewing holds no keys, so it cannot sign the new one.
td97ae94b = What this body carries no longer reads as a signed transaction. tracewing holds no keys, so it cannot sign it again.
ta6e69f9a = This body carries a signed transaction and its bytes have changed. The signatures are over the bytes as they were, and tracewing can neither check them nor make new ones.
t0235648c = The certificate: { $e }
tf542b2f4 = The terminal
tc99726ce = The system proxy: { $e }
t024ec871 = Blocking HTTP/3: { $e }
t00d0cf30 = host:, status:4xx, or text in bodies
t19f89214 = A name, as iPhone test
tf4a9c097 = Filter
tfce439bf = A name, as Phone
tf9d9c09b = A message to send; { $keys } sends it
te3c971c9 = The search index: { $e }
te295872f = Recording again on { $now }. This computer is still sending its traffic to { $was }: turn the system proxy off and on again.
teb8c8bbd = Recording did not start again: { $problem }
tac1ec1d2 = Click the flow to compare #{ $id } with.
t73446a1d = An earlier session: { $name }
td8659d5b = Read-only. Search covers more as its index is finished. Recording goes on.
te2f35d9e = Read-only. Recording goes on.
t317c0453 = Back to live
tf2cc0563 = { $shown } of { $total }
te75cf20c = Recording
tc9390ffa = Hide what came before. Nothing is deleted: the session keeps it, and Sessions is where it goes for good.
t7e813d25 = System proxy off · { $addr }
t29e7dcd0 = System proxy…
t319ad6b9 = System proxy on · { $addr }
tb0605c20 = This computer's traffic comes here. Killed with this on, tracewing leaves the setting behind and the network stops: `tracewing recover` puts it back.
t53d91aad = Apps
t27acf320 = Devices
tfc7bfe37 = All hosts
t838c1d44 = No host has this in its name
t3c2dfe59 = Hidden
tcf9aefb4 = Hosts
t324b6d0b = Newest
t7eb295f4 = No flow fits this filter
tbd00747c = Nothing yet
t61730a16 = Turn on the system proxy above, or point an app at { $addr }. To try it: curl --proxy http://{ $addr } --cacert { $ca } https://example.com/
t3b4853f8 = Beside the list
t23658daf = Below the list
t0fa632ba = Drop it on a place · Esc puts it back
t4b589266 = Traffic
t7b93f551 = Certificate trusted on this computer
tbf8942c3 = Checking the certificate…
t068dcaff = Installing the certificate; the system may ask you to confirm…
t913da735 = Removing the certificate; the system may ask you to confirm…
t4b34beb2 = HTTPS will fail until the certificate is trusted: set up
tc3e1f9c8 = settings.toml was not used: { $problem }
tff1a5018 = A setting was not saved: { $problem }
t04dcff11 = The proxy stopped: { $failure }
te99f9959 = The disk is nearly full: bodies are not being kept
tdbd64fe6 =
    { $n ->
        [one] { $count } flow not recorded
       *[other] { $count } flows not recorded
    }
t249093b0 =
    { $n ->
        [one] { $count } flow without bodies
       *[other] { $count } flows without bodies
    }
te30201f0 =
    { $n ->
        [one] { $count } oldest flow dropped to stay under { $size }
       *[other] { $count } oldest flows dropped to stay under { $size }
    }
t57bf5200 =
    { $n ->
        [one] { $count } flow
       *[other] { $count } flows
    }
t8c2c795e =
    { $n ->
        [one] { $count } hidden, still recorded · show it
       *[other] { $count } hidden, still recorded · show them
    }
t95b6b766 = Export as HAR
t4ab0ecb4 = Shortcuts { $keys }
tea156fdc = Columns
ta367cc19 = Field { $number }, { $wire }
t212b75bf = gRPC, with no .proto to name its fields
t7f1e0096 = Protobuf, with no .proto to name its fields
tb42f6255 = a number
t0487503b = eight bytes
t65b1d004 = bytes
tabee15aa = four bytes
t9b759fb9 = unknown
t88e6aa3c =
    { $n ->
        [one] a message of { $n } field
       *[other] a message of { $n } fields
    }
t23cd498d = As EIP-55 writes it
t59e723b2 = Checksum
te7ec8c71 = none in it, so nothing to check
t5a262a91 = holds
tbc8974e3 = does not hold: a digit is wrong
t2d1b5c44 = Known as
t5fe6920b = nobody: minted from, or burned to
t2bb477cf = the chain's own coin, not a token
t2372877a = An address
td0126fb7 = An event's topic
td02b533f = Event
tc4ef94ce = Calls
t5bdbbcc4 = Answers with
t21c2b1a6 = A call, by its four bytes
tadfc8ac3 = base64url
tf04e6eba = base64
t613f34ea = Hex
t7c7bb5a9 = Percent-encoded
t195b3db9 = Escaped text
t54958ecb = HTML entities
t5dcdd537 = Version
t8433b026 = Variant
t257b8832 = UUID
tf19e9db9 = ULID
t427abd77 = ObjectId
tfc0f33c3 = Counter
te30e0b47 = KSUID
tde5c1a77 = Worker
t64cec188 = Sequence
te00a2b87 = Snowflake
tc6a0fbd2 = JSON, { $n } fields
t06f5832d = JSON, { $n } items
tc69f7572 = JSON in a string
tc7aa0523 = Expires
tb1835ecc = Issued
td10dacd9 = Not before
t9945210a = Algorithm
t1deca496 = State
t338eb632 = expired
t6076261e = JWT
td8d653d6 =
    { $n ->
        [one] { $n } row
       *[other] { $n } rows
    }
tec53a755 = Times
t74dee54c = Names and ids
t1d5a82d9 = Tracing
t500d90ac = The web's own
t18ac79a2 = Networks
t3f5279c5 = Data
taf58dad9 = Encodings
t6d550b01 = Packed
t50973c5a = Tokens and keys
td63c77c9 = Numbers
t109ecadf = Chains
t6edf5796 = Unix seconds
t0d28f4b7 = Unix milliseconds
tc705fe24 = Unix microseconds
t235cb430 = Unix nanoseconds
t402bdff3 = ISO 8601
t3bb6c7ca = ISO 8601, no zone: read as UTC
te55df34c = HTTP-date
t8582ebf1 = Windows FILETIME
t8c35aaf6 = .NET ticks
t4bd1d35a = Seconds since 2001 (Apple)
tc7a82a5d = Seconds since 1904 (HFS+)
td7bc06f2 = Days since 1899 (spreadsheet)
t7e12f48a = Seconds
ta58c7c95 = Trace context
tb4f9b9bb = B3 trace
t122e82a4 = Amazon trace
tb79773df = Server-Timing
t2f58f1e0 = Query fields
te161b66b = Basic authentication
t3868d34d = Cache-Control
t20b704bc = Status code
tbdbabadf = Cookie
teee15f8f = data: URI
t43fe7378 = IPv6
t889c22f2 = A range of addresses
t6d968a53 = A MAC address
t9c53ba7c = As an address
t9fea9931 = As a port
tdfbd7fc3 = JSON
taac68056 = Protobuf
t1a451735 = gzip
t7c5452c4 = zlib
t1dfa0080 = deflate
tcfa9c52a = zstd
t5b0660bd = Brotli
t6da81e5b = JWE, an encrypted token
t309420ff = PASETO
t015e7110 = SAML
tf70119c9 = PEM
t0157a1e6 = A hashed password
te82b53c8 = A hash, by its size
ta9175b27 = As a size
tc31ef85f = In hex and binary
tb67e8e09 = Bits set
t8bdabc70 = this machine
tbe1021ed = a private network
tadacc537 = link-local
t1010cf81 = multicast
t9740f2e4 = a carrier's network
t5fdddd75 = Short
t8d20daf3 = In full
tc1fb8957 = The IPv4 in it
t065b8fa0 = What it is
t18743595 = From
t41f59644 = To
tc31ec050 = How many
t660024a1 = to many, not to one
tc16b0e31 = given by hand, not by the maker
tfda57c3e = in { $amount }
t5f968014 = { $amount } ago
ted680adb = Key wrapped with
t7e4ecbbd = Encrypted with
t79491281 = Key named
tbfad8752 = The rest
tacb4301a = encrypted: it needs the key, which tracewing does not have
t7f5984ef = What it is for
t63aa35b5 = shared key, encrypted
tf7f41fd7 = public key, signed
t5918e3d1 = The claims
t562aaed5 = MD5, or MD4, or NTLM
tf202ac9d = SHA-1, or RIPEMD-160
tc24ef188 = SHA-224
ta2e8a8a4 = SHA-256, or SHA3-256, or BLAKE2s
ta4d7ea95 = SHA-384
t275da5bd = SHA-512, or SHA3-512, or BLAKE2b
te8ff7e65 = Made by
tef91acbf = bcrypt
tfdc8b178 = Rounds
t25e03044 = 2 to the { $cost }
t58f879ce = Memory
t364b5a7c = Passes
t81449775 = In parallel
tefb8e7ad = SHA-256 crypt
t9dcf716c = SHA-512 crypt
tff372c62 = What it holds
td909a26a = How much
tbfdcf758 = { $size } of DER
t867ac265 = Which message
t3890a578 = Made
t6055a346 = Named
t26f8e74e = Trace
taba837e1 = Span
t7e4a1d99 = Sampled
t4e9f3590 = yes
t62366ffa = no
t3cf3f9c3 = Parent span
t3a1b8566 = Began
tcea1562e = Took
t9c9083a2 = Scheme
te2c53cc6 = Port
te0a63e12 = User
t98976d50 = { $n } fields
t7889e7fd = Fragment
t0fb83d07 = Field
td147f96a = Value
t2cc30838 = Password
t83c57deb = never kept
t01f67b47 = kept { $span }
t5af054c4 = kept { $span } by shared caches
tfb7516b7 = by the browser only
t86fe63fe = by anyone
t02076cba = checked every time
t0f7f5bf9 = not used once stale
tc01144d0 = never checked again
t514d8ddb = data: URI, { $media }
