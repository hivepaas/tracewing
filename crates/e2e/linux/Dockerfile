# A Linux desktop's machinery with no desktop: what tracewing changes on a
# user's computer, in a container that may be changed. scripts/e2e_linux.sh
# builds it and runs the tests in it as an ordinary user, who reaches root
# through pkexec as a user of a desktop does.
#
# The same release as the machine that builds the binaries, so that they run
# here as they are.
FROM ubuntu:26.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
        dbus-daemon dbus-bin \
        libglib2.0-bin gsettings-desktop-schemas dconf-gsettings-backend \
        libkf6config-bin \
        libnss3-tools \
        gnome-keyring libsecret-tools \
        nftables \
        polkitd pkexec \
        libxkbcommon0 \
    && rm -rf /var/lib/apt/lists/*

# The browsers S5 opens, as their makers ship them for Ubuntu: Chrome's .deb,
# and Firefox from Mozilla's own repository (Ubuntu's is a snap, which a
# container cannot run).
RUN curl -fsSL --retry 5 -o /tmp/chrome.deb \
        https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb \
    && install -d -m 0755 /etc/apt/keyrings \
    && curl -fsSL --retry 5 -o /etc/apt/keyrings/packages.mozilla.org.asc \
        https://packages.mozilla.org/apt/repo-signing-key.gpg \
    && echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
        > /etc/apt/sources.list.d/mozilla.list \
    && printf 'Package: *\nPin: origin packages.mozilla.org\nPin-Priority: 1000\n' \
        > /etc/apt/preferences.d/mozilla \
    && apt-get update && apt-get install -y --no-install-recommends /tmp/chrome.deb firefox \
    && rm -rf /var/lib/apt/lists/* /tmp/chrome.deb
ENV TRACEWING_E2E_BROWSERS="chrome firefox"

# The user the tests run as, and what lets it through pkexec without the
# password dialog a desktop would show: the rule says yes to it alone.
RUN useradd --create-home --shell /bin/bash tester
COPY 49-tracewing-e2e.rules /etc/polkit-1/rules.d/49-tracewing-e2e.rules

# What says this machine may be changed. Only root can make it, and only this
# image has it.
RUN touch /etc/tracewing-e2e-throwaway

COPY run.sh /usr/local/bin/tracewing-e2e-run
COPY session.sh /usr/local/bin/tracewing-e2e-session
ENTRYPOINT ["/usr/local/bin/tracewing-e2e-run"]
