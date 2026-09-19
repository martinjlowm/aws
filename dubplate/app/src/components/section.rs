//! The heading a card is announced by.
//!
//! Every card on the page opens the same way: a mono eyebrow saying which step
//! this is, and under it the line saying what the step does. Written out three
//! times it drifted, and had: two of them were headings at the display size and
//! the third was a `span` at body size in the body face, so the page read as one
//! step, a subheading and another step. One component holds all of them to one
//! size, one face and one rhythm.

use leptos::prelude::*;

/// An eyebrow and a title, as one heading.
///
/// Use this wherever the card's header is not itself a control. Settings cannot:
/// its header is the button that opens it, and a heading may not sit inside a
/// button, so it puts the button inside the heading and reaches for
/// [`SectionTitle`] on its own.
#[component]
pub fn SectionHeading(
    /// Which step this is, or what the section is.
    eyebrow: &'static str,
    /// What it does, which is the heading proper.
    title: &'static str,
) -> impl IntoView {
    view! {
        <h2 class="section-heading">
            <SectionTitle eyebrow=eyebrow title=title />
        </h2>
    }
}

/// The eyebrow and the title, without the element that makes them a heading.
#[component]
pub fn SectionTitle(eyebrow: &'static str, title: &'static str) -> impl IntoView {
    view! {
        <span class="block">
            <span class="label block">{eyebrow}</span>
            <span class="section-title block">{title}</span>
        </span>
    }
}
