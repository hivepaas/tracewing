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
    /// A Mac app that ignores proxy settings altogether.
    MacApp,
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
        Self::MacApp,
        Self::Container,
    ];

    /// The ways in that are worth offering on this computer. A Mac app is
    /// taken by a macOS extension, and on Linux or Windows there is nothing
    /// behind that answer, so it is not asked.
    pub fn here() -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|target| cfg!(target_os = "macos") || *target != Self::MacApp)
            .collect()
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
            Self::MacApp => tr!("A Mac app that ignores the proxy"),
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
    MacApps,
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
        Target::MacApp => with(Way::Apps, Trust::SystemStore, &[Card::MacApps]),
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

    /// A phone's cards are the guides for it, and a Mac app's is the one that
    /// names apps: a route shows what has something to say about it and
    /// nothing else, because the pile of cards is what made Set up hard to
    /// read.
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

    #[test]
    fn a_route_shows_the_cards_that_are_about_it() {
        assert_eq!(
            route(Target::Phone).cards,
            vec![Card::OtherDevices, Card::Android, Card::Simulators]
        );
        assert_eq!(route(Target::MacApp).cards, vec![Card::MacApps]);
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

    /// A Mac app is only offered where macOS could take it.
    #[test]
    fn what_is_asked_here_is_what_this_computer_can_do() {
        let here = Target::here();
        assert_eq!(
            here.contains(&Target::MacApp),
            cfg!(target_os = "macos"),
            "an answer with nothing behind it is worse than one fewer answer"
        );
        assert!(here.contains(&Target::Website));
    }
}
