//! The slow network, from the Rules screen: on or off, a preset, or numbers of
//! one's own. Applies from the next request, and is kept in `throttle.toml`.

use crate::{
    colors::colors,
    engine::THROTTLE_FILE,
    input::TextInput,
    traffic::{Screen, Traffic},
    widgets::{Reachable, eyebrow},
};
use gpui::{AppContext as _, Context, Div, Entity, FontWeight, div, prelude::*, px, rgb};
use tracewing_i18n::tr;
use tracewing_proxy::throttle::{Conditions, PRESETS};

/// The fields for conditions of one's own.
pub struct NetworkForm {
    pub latency: Entity<TextInput>,
    pub down: Entity<TextInput>,
    pub up: Entity<TextInput>,
    pub loss: Entity<TextInput>,
    pub hosts: Entity<TextInput>,
    pub problem: Option<String>,
}

fn number_text(n: u32) -> String {
    if n == 0 { String::new() } else { n.to_string() }
}

/// A field's number: empty is none.
fn number(text: &str, what: &str) -> Result<u32, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(0);
    }
    text.parse().map_err(|_| {
        tr!(
            "{what}: {text} is not a whole number",
            what = what,
            text = text
        )
    })
}

/// Conditions from the fields, as a person typed them.
pub fn conditions_from(
    latency: &str,
    down: &str,
    up: &str,
    loss: &str,
    hosts: &str,
) -> Result<Conditions, String> {
    let conditions = Conditions {
        on: true,
        name: "Custom".to_owned(),
        latency_ms: number(latency, &tr!("Latency"))?,
        down_kbps: number(down, &tr!("Down"))?,
        up_kbps: number(up, &tr!("Up"))?,
        loss_percent: number(loss, &tr!("Loss"))?,
        hosts: hosts
            .split([',', ' ', '\n'])
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .map(str::to_owned)
            .collect(),
    };
    conditions.check().map_err(|e| format!("{e:#}"))?;
    if !conditions.slows() {
        return Err(tr!("Give a latency, a speed or a loss to slow it down by"));
    }
    Ok(conditions)
}

impl NetworkForm {
    fn new(conditions: &Conditions, cx: &mut Context<Traffic>) -> Self {
        let field = |hint: fn() -> String, text: String, cx: &mut Context<Traffic>| {
            cx.new(|cx| {
                let mut input = TextInput::said(hint, cx);
                input.set_text(&text, text.len()..text.len(), cx);
                input
            })
        };
        Self {
            latency: field(
                || tr!("Latency, ms"),
                number_text(conditions.latency_ms),
                cx,
            ),
            down: field(
                || tr!("Down, kbit/s"),
                number_text(conditions.down_kbps),
                cx,
            ),
            up: field(|| tr!("Up, kbit/s"), number_text(conditions.up_kbps), cx),
            loss: field(|| tr!("Loss, %"), number_text(conditions.loss_percent), cx),
            hosts: field(
                || tr!("Only these hosts, like *.example.com; all if empty"),
                conditions.hosts.join(", "),
                cx,
            ),
            problem: None,
        }
    }

    fn fill(&self, conditions: &Conditions, cx: &mut Context<Traffic>) {
        for (input, text) in [
            (&self.latency, number_text(conditions.latency_ms)),
            (&self.down, number_text(conditions.down_kbps)),
            (&self.up, number_text(conditions.up_kbps)),
            (&self.loss, number_text(conditions.loss_percent)),
            (&self.hosts, conditions.hosts.join(", ")),
        ] {
            input.update(cx, |input, cx| {
                input.set_text(&text, text.len()..text.len(), cx)
            });
        }
    }
}

impl Traffic {
    fn network_form(&mut self, cx: &mut Context<Self>) -> &mut NetworkForm {
        if self.network_form.is_none() {
            let conditions = self.engine.network.conditions();
            self.network_form = Some(NetworkForm::new(&conditions, cx));
        }
        self.network_form.as_mut().expect("just made")
    }

    pub fn network_conditions(&self) -> Conditions {
        self.engine.network.conditions()
    }

    /// From the next request on, and kept for the next start.
    pub fn set_network(&mut self, conditions: Conditions, cx: &mut Context<Self>) {
        self.engine.network.set(conditions.clone());
        let saved = conditions.save(&self.data_dir.join(THROTTLE_FILE));
        let form = self.network_form(cx);
        form.problem = saved
            .err()
            .map(|e| tr!("Not kept for next time: {e}", e = format!("{e:#}")));
        if let Some(form) = &self.network_form {
            form.fill(&conditions, cx);
        }
        cx.notify();
    }

    /// A preset, for the hosts already chosen.
    pub fn choose_network(&mut self, preset: &str, cx: &mut Context<Self>) {
        let Some(mut conditions) = Conditions::preset(preset) else {
            return;
        };
        conditions.hosts = self.network_conditions().hosts;
        self.set_network(conditions, cx);
    }

    pub fn toggle_network(&mut self, cx: &mut Context<Self>) {
        let mut conditions = self.network_conditions();
        if !conditions.on && !conditions.slows_when_on() {
            return self.choose_network(PRESETS[1].0, cx);
        }
        conditions.on = !conditions.on;
        self.set_network(conditions, cx);
    }

    /// Conditions of one's own, from the fields.
    pub fn apply_network_form(&mut self, cx: &mut Context<Self>) {
        let form = self.network_form(cx);
        let text = |input: &Entity<TextInput>| input.read(cx).text().to_owned();
        let read = conditions_from(
            &text(&form.latency),
            &text(&form.down),
            &text(&form.up),
            &text(&form.loss),
            &text(&form.hosts),
        );
        match read {
            Ok(conditions) => self.set_network(conditions, cx),
            Err(problem) => {
                self.network_form(cx).problem = Some(problem);
                cx.notify();
            }
        }
    }

    /// Fills the fields, to type into them. For tests.
    #[doc(hidden)]
    pub fn fill_network_form(&mut self, fields: [&str; 5], cx: &mut Context<Self>) {
        let form = self.network_form(cx);
        let inputs = [
            form.latency.clone(),
            form.down.clone(),
            form.up.clone(),
            form.loss.clone(),
            form.hosts.clone(),
        ];
        for (input, text) in inputs.iter().zip(fields) {
            input.update(cx, |input, cx| {
                input.set_text(text, text.len()..text.len(), cx)
            });
        }
    }

    /// The Rules screen's slow network.
    pub(crate) fn render_network(&mut self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let conditions = self.network_conditions();
        let form = self.network_form(cx);
        let (latency, down, up, loss, hosts) = (
            form.latency.clone(),
            form.down.clone(),
            form.up.clone(),
            form.loss.clone(),
            form.hosts.clone(),
        );
        let problem = form.problem.clone();
        let on = conditions.on;
        let chip = |id: &'static str, label: String, chosen: bool| {
            div()
                .id(id)
                .h(px(30.))
                .px(px(12.))
                .rounded(px(15.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(px(12.5))
                .font_weight(FontWeight::BOLD)
                .when(chosen, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!chosen, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        let presets = PRESETS.iter().map(|&(name, ..)| {
            chip(name, name.to_owned(), on && conditions.name == name)
                .on_click(cx.listener(move |this, _, _, cx| this.choose_network(name, cx)))
        });
        // Labelled above: a field that holds a number shows no hint.
        let field = |label: String, input: Entity<TextInput>, wide: bool| {
            div()
                .flex()
                .flex_col()
                .gap(px(3.))
                .when(wide, |d| d.flex_1().min_w(px(260.)))
                .when(!wide, |d| d.w(px(130.)))
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(rgb(c.muted))
                        .child(label),
                )
                .child(
                    div()
                        .h(px(36.))
                        .px(px(12.))
                        .rounded(px(12.))
                        .flex()
                        .items_center()
                        .bg(rgb(c.input))
                        .child(input),
                )
        };
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .child(eyebrow(tr!("Slow network")))
                    .child(
                        div()
                            .id("network-on")
                            .w(px(44.))
                            .h(px(24.))
                            .p(px(3.))
                            .rounded(px(12.))
                            .flex()
                            .cursor_pointer()
                            .reachable()
                            .when(on, |d| d.bg(rgb(c.accent)).justify_end())
                            .when(!on, |d| d.bg(rgb(c.input)))
                            .child(div().size(px(18.)).rounded(px(9.)).bg(rgb(if on {
                                c.on_accent
                            } else {
                                c.muted
                            })))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_network(cx))),
                    )
                    .child(div().text_color(rgb(c.muted)).child(if on {
                        conditions.summary()
                    } else {
                        tr!("Off: traffic goes as fast as it can")
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(6.))
                    .children(presets)
                    .child(
                        chip("Custom", tr!("Custom"), on && conditions.name == "Custom")
                            .on_click(cx.listener(|this, _, _, cx| this.apply_network_form(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_end()
                    .gap(px(8.))
                    .child(field(tr!("Latency, ms"), latency, false))
                    .child(field(tr!("Down, kbit/s"), down, false))
                    .child(field(tr!("Up, kbit/s"), up, false))
                    .child(field(tr!("Loss, %"), loss, false))
                    .child(field(tr!("Only these hosts"), hosts, true))
                    .child(
                        chip("network-apply", tr!("Apply"), false)
                            .on_click(cx.listener(|this, _, _, cx| this.apply_network_form(cx))),
                    ),
            )
            .child(div().text_size(px(12.)).text_color(rgb(c.muted)).child(tr!(
                "Applies from the next request. Latency is added to each round trip; the \
                 speed is a link that every slowed flow shares; a lost piece waits to be \
                 sent again, as TCP does. Hosts left alone are not slowed."
            )))
            .children(problem.map(|p| div().text_color(rgb(c.error)).child(p)))
    }

    /// In the toolbar while the network is slow, to see it and to change it.
    pub(crate) fn render_network_chip(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let conditions = self.network_conditions();
        if !conditions.on {
            return None;
        }
        let c = colors();
        Some(
            div()
                .id("network-chip")
                .h(px(40.))
                .px(px(16.))
                .rounded(px(20.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .bg(rgb(c.warn_ground))
                .text_color(rgb(c.warn))
                .font_weight(FontWeight::BOLD)
                .child(conditions.summary())
                .on_click(cx.listener(|this, _, _, cx| this.set_screen(Screen::Rules, cx))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_become_conditions_or_say_what_is_wrong() {
        let conditions =
            conditions_from("300", "1000", "", "2", "*.example.com, api.test").unwrap();
        assert_eq!(
            (
                conditions.latency_ms,
                conditions.down_kbps,
                conditions.up_kbps,
                conditions.loss_percent
            ),
            (300, 1000, 0, 2)
        );
        assert_eq!(conditions.hosts, ["*.example.com", "api.test"]);
        assert_eq!(conditions.name, "Custom");
        assert!(
            conditions_from("fast", "", "", "", "")
                .unwrap_err()
                .contains("Latency")
        );
        assert!(
            conditions_from("", "", "", "120", "")
                .unwrap_err()
                .contains("more than all")
        );
        assert!(
            conditions_from("", "", "", "", "")
                .unwrap_err()
                .contains("slow it down")
        );
    }
}
