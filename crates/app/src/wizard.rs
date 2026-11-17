//! The wizard's knowledge: what somebody wants to capture, and the shortest
//! way to it.
//!
//! Set up used to show every way in at once — three numbered steps and seven
//! cards, whether or not any of them had anything to do with what somebody
//! was trying to see. This is the other half of
//! [setup-flow.md](../../../docs/design/setup-flow.md)'s two surfaces: the
//! configuration page ([`crate::layout::Way`]) is for somebody who knows
//! which way in they want, and the wizard is for somebody who knows only
//! what they are trying to capture.
//!
//! What is here is the **routing**, as plain functions with tests: a target
//! names a way in, what has to trust tracewing, and which of Set up's cards
//! have anything to say about it. None of it draws anything, because the
//! question "does a Java program need the system proxy" should be answerable
//! without a window — and because the answers are the part that will be
//! argued about.

use crate::layout::Way;
use serde::{Deserialize, Serialize};
use tracewing_i18n::tr;

/// What somebody says they are trying to capture.
///
/// These are not the ways in rearranged: they are the things people arrive
/// wanting, in their own terms. Several of them come out at the same way in —
/// `curl`, Node and Python are all a terminal — and that is the point. The
/// wizard exists so that nobody has to know that in advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    /// A site in a browser, which is what most people come for.
    Website,
    /// `curl`, `git`, a shell script: what reads the proxy variables.
    CommandLine,
    /// Node, Deno or Bun, which need a version that reads them.
    Node,
    /// Python, whose trust comes from a file of roots.
    Python,
    /// A Java program, which reads neither the system proxy nor the
    /// variables, and keeps an authority store of its own.
    Java,
    /// A phone or a tablet.
    Phone,
    /// A program on this computer that ignores proxy settings altogether.
    ///
    /// One answer on all three systems, because what somebody arrives
    /// wanting is the same on each: macOS hands the flows over through its
    /// network extension, Windows and Linux by route through an adapter of
    /// tracewing's. Which of the two is underneath is exactly the kind of
    /// thing the wizard exists so that nobody has to know.
    ///
    /// The alias is what an older `layout.toml` holds: this answer was
    /// `mac-app` while macOS was the only system that had it, and a kept
    /// answer that no longer parses would put the question back in front of
    /// somebody who had already answered it.
    #[serde(alias = "mac-app")]
    LocalApp,
    /// Something in Docker or Podman.
    Container,
}

impl Target {
    /// In the order people arrive wanting them: the browser first, because it
    /// is most of the asking, and the awkward cases after.
    pub const ALL: [Self; 8] = [
        Self::Website,
        Self::CommandLine,
        Self::Node,
        Self::Python,
        Self::Java,
        Self::Phone,
        Self::LocalApp,
        Self::Container,
    ];

    /// The answers worth offering on this computer.
    ///
    /// Nothing is left out today. The one answer that used to be left out
    /// was the local way in, when macOS was the only system that could take
    /// a chosen program's traffic; it reaches [`Way::Apps`] on all three now,
    /// and leaving it out on two of them made the tab that Settings draws
    /// everywhere findable only by somebody who already knew to look there.
    ///
    /// The seam stays although it filters nothing: the next answer that is
    /// true of one system only has to be left out somewhere, and a caller
    /// reading [`Self::ALL`] straight would not know to.
    pub fn here() -> Vec<Self> {
        Self::ALL.to_vec()
    }

    /// As somebody would say it, in the first person, because the question is
    /// "what do you want to capture".
    pub fn name(self) -> String {
        match self {
            Self::Website => tr!("A website in my browser"),
            Self::CommandLine => tr!("curl, git, a script"),
            // The runtimes' own names: not translated.
            Self::Node => "Node, Deno, Bun".to_owned(),
            Self::Python => "Python".to_owned(),
            Self::Java => tr!("A Java program"),
            Self::Phone => tr!("My phone"),
            // The word the reader's own system uses, as `Way::name` does
            // for the tab this answer leads to. A row of pills is scanned
            // rather than read, and somebody on Windows looking for where to
            // capture a service or an installer does not stop at "app".
            Self::LocalApp if cfg!(windows) => tr!("A program that ignores the proxy"),
            Self::LocalApp => tr!("An app that ignores the proxy"),
            Self::Container => tr!("Something in Docker"),
        }
    }
}

/// What has to trust tracewing's authority before HTTPS can be read, and who
/// does the trusting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trust {
    /// This computer's own store, which Set up installs into.
    SystemStore,
    /// Nothing is installed anywhere: the terminal sets `SSL_CERT_FILE`,
    /// `NODE_EXTRA_CA_CERTS` and `REQUESTS_CA_BUNDLE` for what it starts.
    TheTerminal,
    /// A store of Java's own, which tracewing writes and hands over with the
    /// flags that point a JVM at it.
    JavaItself,
    /// The phone, by the guide for it.
    ThePhone,
    /// Whatever the image brings, which tracewing cannot know.
    Unknown,
}

/// A card of Set up's that has something to say about a route. The wizard
/// shows these and leaves the rest off the page — the pile of cards was the
/// thing that made Set up hard to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Card {
    Browsers,
    Terminal,
    OtherDevices,
    Android,
    Simulators,
    /// The chosen programs and the switch over them, whichever of the two
    /// local ways in this system has behind it.
    LocalApps,
    /// Blocking UDP port 443, so apps that speak HTTP/3 fall back to HTTP/2
    /// and come through the proxy at all. Not a way in and not a setting:
    /// it is the companion of the system proxy, and the only place in the
    /// window that offers it.
    Http3,
}

/// The shortest path to a target: one way in, one thing to trust, and the
/// cards worth reading for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub way: Way,
    pub trust: Trust,
    pub cards: Vec<Card>,
}

/// What to do about a target, from
/// [capturing.md](../../../docs/capturing.md)'s table.
///
/// A terminal answers four of the eight, which is why the wizard is worth
/// having: three of those four would otherwise have somebody turning the
/// system proxy on and finding that it changed nothing.
pub fn route(target: Target) -> Route {
    let with = |way, trust, cards: &[Card]| Route {
        way,
        trust,
        cards: cards.to_vec(),
    };
    match target {
        // The system proxy first and a browser of its own beside it: most
        // browsers follow the system's setting, and Safari follows nothing
        // else.
        Target::Website => with(
            Way::SystemProxy,
            Trust::SystemStore,
            // HTTP/3 belongs to this answer and no other: a browser is what
            // speaks it, and an app that does goes around the proxy
            // altogether.
            &[Card::Browsers, Card::Http3, Card::Terminal],
        ),
        // The terminal brings its own trust, so nothing has to be installed
        // on this computer at all.
        Target::CommandLine | Target::Node | Target::Python => {
            with(Way::Terminal, Trust::TheTerminal, &[Card::Terminal])
        }
        // Java reads neither the system proxy nor the variables. The system
        // proxy is still the way in — its flags point at the same port — but
        // the trust is its own, and tracewing cannot do that part yet.
        Target::Java => with(Way::SystemProxy, Trust::JavaItself, &[Card::Terminal]),
        Target::Phone => with(
            Way::OtherDevices,
            Trust::ThePhone,
            &[Card::OtherDevices, Card::Android, Card::Simulators],
        ),
        // The same store as the system proxy's, and for the same reason on
        // either machinery: whatever hands the connection over, the program
        // at the end of it still checks the certificate it is given.
        Target::LocalApp => with(Way::Apps, Trust::SystemStore, &[Card::LocalApps]),
        // A terminal inside the container; the daemon's own traffic is a
        // different question, and what the image trusts is not ours to say.
        Target::Container => with(Way::Terminal, Trust::Unknown, &[Card::Terminal]),
    }
}

/// Whether tracewing can set the trust up itself, or can only say what to do.
///
/// The honest answer matters more here than anywhere else on the page: a
/// wizard that claims a step is done when it has not been done sends
/// somebody to look for a bug in their program.
pub fn tracewing_can_trust(trust: Trust) -> bool {
    match trust {
        // Java's is tracewing's to write: a PKCS#12 of its own, in the data
        // directory, which touches nothing of the user's and needs no JDK.
        Trust::SystemStore | Trust::TheTerminal | Trust::JavaItself => true,
        Trust::ThePhone | Trust::Unknown => false,
    }
}

/// Firefox, or one of the browsers built from it, which keep an authority
/// store of their own in each profile.
pub fn is_firefox(program: &str) -> bool {
    let program = program.to_lowercase();
    ["firefox", "librewolf", "waterfox"]
        .iter()
        .any(|name| program.contains(name))
}

/// What to do about a program that turned tracewing's certificate down: the
/// target whose route sets its trust up, and a line on where that program
/// looks for authorities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    pub target: Target,
    pub says: String,
}

/// The fix for a program, told by its name or its file, or by its being on
/// another device. `None` when nothing is known of it, or it is nothing in
/// particular: then all there is to say is the general advice, and that a
/// program that pins its certificates will not be read by anyone.
///
/// The names are matched loosely, because they come as the system gives
/// them: a file (`curl`, `python3.12`, `node.exe`), a path, or a macOS
/// bundle (`org.mozilla.firefox`).
pub fn fix_for(program: Option<&str>, from_device: bool) -> Option<Fix> {
    let fix = |target, says| Some(Fix { target, says });
    if from_device {
        return fix(
            Target::Phone,
            tr!(
                "A phone trusts tracewing once its CA is installed there. On Android, most apps trust only the system's authorities, not one a user adds."
            ),
        );
    }
    let program = program?.to_lowercase();
    let file = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&program)
        .trim_end_matches(".exe");
    let any = |names: &[&str]| names.iter().any(|name| program.contains(name));
    let starts = |names: &[&str]| names.iter().any(|name| file.starts_with(name));
    // Exactly, or with a version after it: `node`, `node22`, `python3.12`,
    // but not `bundle` or `gitkraken`.
    let named = |names: &[&str]| {
        names.iter().any(|name| {
            file.strip_prefix(name).is_some_and(|rest| {
                rest.is_empty()
                    || rest.starts_with(|c: char| c.is_ascii_digit() || c == '.' || c == '-')
            })
        })
    };
    if is_firefox(&program) {
        fix(
            Target::Website,
            tr!(
                "Firefox keeps its own authorities in each profile, apart from this computer's. Set up installs tracewing's CA into them."
            ),
        )
    } else if any(&[
        "chrome", "chromium", "msedge", "edge", "brave", "opera", "vivaldi", "safari",
    ]) {
        fix(
            Target::Website,
            tr!(
                "This browser trusts the authorities installed on this computer. Set up installs tracewing's CA there."
            ),
        )
    } else if named(&["node", "deno", "bun"]) {
        fix(
            Target::Node,
            tr!(
                "Node, Deno and Bun carry their own authorities. Started from tracewing's terminal, they are told to trust its CA as well."
            ),
        )
    } else if named(&["python", "pip", "uv"]) {
        fix(
            Target::Python,
            tr!(
                "Python trusts the file of authorities it is pointed to, which tracewing's terminal points at one with tracewing's CA. Apple's own Python ignores it."
            ),
        )
    } else if named(&["java", "javaw"]) {
        fix(
            Target::Java,
            tr!(
                "Java trusts only a store of its own. Set up writes one with tracewing's CA, and gives the flags that point Java at it."
            ),
        )
    } else if starts(&["docker", "podman", "containerd", "com.docker"]) {
        fix(
            Target::Container,
            tr!(
                "What runs in a container trusts what its image brings, so tracewing's CA has to be added to the image."
            ),
        )
    } else if named(&["curl", "wget", "git", "http", "https", "aws"]) {
        fix(
            Target::CommandLine,
            tr!(
                "Started from tracewing's terminal, a command-line program is told to trust tracewing's CA."
            ),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every target leads somewhere that exists. A target whose way in is not
    /// built would be a wizard sending somebody to a tab with nothing in it.
    #[test]
    fn every_target_has_a_way_in_that_is_built() {
        for target in Target::ALL {
            let route = route(target);
            assert!(
                Way::ALL.contains(&route.way),
                "{:?} goes to a way in that is not in the tab strip",
                target
            );
        }
    }

    /// The four that a terminal answers, which is the wizard's whole reason
    /// for existing: without it, three of these four would have somebody
    /// turning the system proxy on and finding that nothing changed.
    #[test]
    fn a_terminal_answers_the_four_that_the_system_proxy_cannot() {
        let by_terminal: Vec<Target> = Target::ALL
            .into_iter()
            .filter(|t| route(*t).way == Way::Terminal)
            .collect();
        assert_eq!(
            by_terminal,
            vec![
                Target::CommandLine,
                Target::Node,
                Target::Python,
                Target::Container
            ]
        );
    }

    /// Java reads neither the system's store nor the variables, so its way in
    /// is the proxy and its trust is its own file — which tracewing writes,
    /// since `tracewing_system::java` landed.
    #[test]
    fn java_is_the_proxy_with_a_store_of_its_own() {
        let route = route(Target::Java);
        assert_eq!(route.way, Way::SystemProxy);
        assert_eq!(route.trust, Trust::JavaItself);
        assert!(
            tracewing_can_trust(route.trust),
            "tracewing writes that store now, so the step is not somebody else's"
        );
    }

    /// What tracewing can do itself, and what it can only tell somebody to
    /// do. Written as a list so that making one of them automatic is a change
    /// to this test and not a silent one.
    #[test]
    fn tracewing_trusts_for_three_of_the_five_and_says_so_for_the_rest() {
        let ours: Vec<Trust> = [
            Trust::SystemStore,
            Trust::TheTerminal,
            Trust::JavaItself,
            Trust::ThePhone,
            Trust::Unknown,
        ]
        .into_iter()
        .filter(|trust| tracewing_can_trust(*trust))
        .collect();
        assert_eq!(
            ours,
            vec![Trust::SystemStore, Trust::TheTerminal, Trust::JavaItself]
        );
    }

    /// Blocking UDP 443 is offered where it helps and nowhere else. It is
    /// the window's only surface for that block, so dropping it from a route
    /// is the kind of thing that loses a feature quietly.
    #[test]
    fn http3_is_offered_to_the_browser_and_to_nothing_else() {
        let with_http3: Vec<Target> = Target::ALL
            .into_iter()
            .filter(|t| route(*t).cards.contains(&Card::Http3))
            .collect();
        assert_eq!(with_http3, vec![Target::Website]);
    }

    /// A phone's cards are the guides for it, and the local way in's is the
    /// one that names programs: a route shows what has something to say
    /// about it and nothing else, because the pile of cards is what made Set
    /// up hard to read.
    #[test]
    fn a_route_shows_the_cards_that_are_about_it() {
        assert_eq!(
            route(Target::Phone).cards,
            vec![Card::OtherDevices, Card::Android, Card::Simulators]
        );
        assert_eq!(route(Target::LocalApp).cards, vec![Card::LocalApps]);
        assert!(
            !route(Target::Website).cards.contains(&Card::OtherDevices),
            "a website in a browser has nothing to do with another device"
        );
        for target in Target::ALL {
            assert!(
                !route(target).cards.is_empty(),
                "{target:?} would show a question and then an empty page"
            );
        }
    }

    /// What is asked is what this computer can do: an answer with nothing
    /// behind it is worse than one fewer answer.
    ///
    /// Every answer is offered on every system today, which is the whole of
    /// what this says. It is written as an equality rather than as a count
    /// so that the next answer that is true of one system only cannot be
    /// filtered out quietly: filtering is what left the local way in
    /// unaskable on two systems for as long as it did.
    #[test]
    fn what_is_asked_here_is_what_this_computer_can_do() {
        assert_eq!(
            Target::here(),
            Target::ALL.to_vec(),
            "an answer is being left out; the test below says whether the \
             tab it leads to was left out with it"
        );
    }

    /// The local way in is reachable from the question wherever it does
    /// anything, which is the gap this test exists for: Settings drew the
    /// `Way::Apps` tab on all three systems while only macOS had an answer
    /// that led to it, so on Windows and Linux the one way in that takes an
    /// app ignoring the proxy could be found only by somebody who already
    /// knew to look in Settings.
    ///
    /// Neither surface's own tests could see it. This one reads both: the
    /// question on one side, and on the other what `settings_page` puts in
    /// that tab -- the route-capturing switch where this machine captures by
    /// route, and the network extension on macOS.
    #[test]
    fn the_local_way_in_has_an_answer_wherever_it_does_anything() {
        let asked = Target::here()
            .into_iter()
            .any(|target| route(target).way == Way::Apps);
        let in_the_tab = crate::local_apps::Support::here()
            != crate::local_apps::Support::NotOnThisSystem
            || cfg!(target_os = "macos");
        assert_eq!(
            asked, in_the_tab,
            "the Apps tab and Set up's question disagree about whether this \
             system can take a program that ignores the proxy"
        );
    }

    /// An answer kept before the rename is still read. `layout.toml` holds
    /// the answer by name, and this one was `mac-app` for as long as macOS
    /// was the only system that had it: a kept answer that no longer parsed
    /// would put the question back in front of somebody who had answered it,
    /// which is the one way this rename could be felt by anybody.
    #[test]
    fn an_answer_kept_before_the_rename_is_still_read() {
        #[derive(serde::Deserialize)]
        struct Kept {
            capture_target: Option<Target>,
        }
        let old: Kept = toml::from_str("capture_target = \"mac-app\"").unwrap();
        assert_eq!(old.capture_target, Some(Target::LocalApp));
        let now: Kept = toml::from_str("capture_target = \"local-app\"").unwrap();
        assert_eq!(now.capture_target, Some(Target::LocalApp));
    }

    #[test]
    fn a_program_that_turned_the_certificate_down_is_sent_where_its_trust_is_set_up() {
        let target = |program: &str| fix_for(Some(program), false).map(|fix| fix.target);
        for (program, expected) in [
            ("firefox", Some(Target::Website)),
            ("org.mozilla.firefox", Some(Target::Website)),
            ("/Applications/Google Chrome.app", Some(Target::Website)),
            ("msedge.exe", Some(Target::Website)),
            ("node", Some(Target::Node)),
            ("C:\\Program Files\\nodejs\\node.exe", Some(Target::Node)),
            ("/usr/bin/python3.12", Some(Target::Python)),
            ("java", Some(Target::Java)),
            ("/usr/bin/curl", Some(Target::CommandLine)),
            ("git-remote-https", Some(Target::CommandLine)),
            ("com.docker.backend", Some(Target::Container)),
            ("spotify", None),
            ("gitkraken", None),
            ("bundle", None),
            // A program called javac is not the JVM.
            ("javac", None),
        ] {
            assert_eq!(target(program), expected, "{program}");
        }
        assert_eq!(fix_for(None, false), None, "nothing known, nothing to say");
        assert!(is_firefox("org.mozilla.firefox") && is_firefox("LibreWolf"));
        assert!(!is_firefox("chrome"));
        assert_eq!(
            fix_for(Some("curl"), true).map(|fix| fix.target),
            Some(Target::Phone),
            "what comes from another device is that device's to trust"
        );
    }
}
