//! The heading a card is announced by.
//!
//! Every card on the page opens the same way: a mono eyebrow naming what the
//! card is about, and under it the line saying what it does. Written out three
//! times it drifted, and had: two of them were headings at the display size and
//! the third was a `span` at body size in the body face, so the page read as a
//! heading, a subheading and another heading. One component holds all of them to
//! one size, one face and one rhythm.
//!
//! The eyebrows named steps once, and stopped: the settings are read when a
//! track is dropped rather than after it, so numbering them second put them
//! after the thing they decide.

use leptos::prelude::*;

/// An eyebrow and a title, as one heading.
///
/// Use this wherever the card's header is not itself a control. Settings cannot:
/// its header is the button that opens it, and a heading may not sit inside a
/// button, so it puts the button inside the heading and reaches for
/// [`SectionTitle`] on its own.
#[component]
pub fn SectionHeading(
    /// What the section is about.
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
pub fn SectionTitle(
    eyebrow: &'static str,
    title: &'static str,
    /// A control belonging to the heading, drawn level with the title.
    ///
    /// Given a row of the grid rather than the whole heading to sit beside: an
    /// eyebrow is a line of mono above the title, and a control centred on the
    /// pair lands in the gap between the two rather than on the line it acts on.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <span class="grid w-full grid-cols-[1fr_auto] items-center gap-x-5">
            <span class="label col-start-1 row-start-1 block">{eyebrow}</span>
            <span class="section-title col-start-1 row-start-2 block">{title}</span>
            {children
                .map(|children| {
                    view! {
                        <span class="section-control col-start-2 row-start-2 flex items-center gap-4">
                            {children()}
                        </span>
                    }
                })}
        </span>
    }
}
