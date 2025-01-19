use maud::{html, Markup, DOCTYPE};

pub fn index() -> Markup {
    let content = html! {};

    page("You're Rostra!", content)
}

/// A basic header with a dynamic `page_title`.
pub(crate) fn head(page_title: &str) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en";
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            link rel="stylesheet" type="text/css" href="/assets/style.css";
            link rel="stylesheet" type="text/css" href="/assets/style-htmx-send-error.css";
            title { (page_title) }
        }
    }
}

pub(crate) fn header() -> Markup {
    html! {
        header ."" {
                div  { "Your Rostra" }
        }
    }
}

/// A static footer.
pub(crate) fn footer() -> Markup {
    html! {
        script src="https://unpkg.com/htmx.org@1.9.12" {};
        script src="https://unpkg.com/htmx.org@1.9.12/dist/ext/response-targets.js" {};
        script type="module" src="/assets/script.js" {};
        script type="module" src="/assets/script-htmx-send-error.js" {};
    }
}

pub fn page(title: &str, content: Markup) -> Markup {
    html! {
        (head(title))
        body ."o-body" {
            // div #"gray-out-page" ."fixed inset-0 send-error-hidden"  {
            //     div ."relative z-50 bg-white mx-auto max-w-sm p-10 flex flex-center flex-col gap-2" {
            //         p { "Connection error" }
            //         button ."rounded bg-red-700 text-white px-2 py-1" hx-get="/" hx-target="body" hx-swap="outerHTML" { "Reload" }
            //     }
            //     div ."inset-0 absolute z-0 bg-gray-500 opacity-50" {}
            // }
            div ."o-pageLayout" {

                // (header())
                nav ."o-navBar" {
                    div ."o-navBar__list" {
                        a ."o-navBar__item" href="/" { "Home" }
                        a ."o-navBar__item" href="/" { "Something" }
                    }
                }

                main ."o-mainBar" {
                    div ."o-mainBar__item" {
                        (post("dpc", "Cool stuff"))
                    }

                    div ."o-mainBar__item" {
                        (post("someone", "Some other cool stuff"))
                    }

                }

                div ."o-sideBar" {
                    "side bar"
                }
                (footer())
            }
        }
    }
}

pub fn post(username: &str, content: &str) -> Markup {
    html! {
        article ."m-postOverview" {
            div ."m-postOverview__main" {
                img ."m-postOverview__userImage" src="https://avatars.githubusercontent.com/u/9209?v=4" { }

                div ."m-postOverview__contentSide" {
                    header .".m-postOverview__header" {
                        span ."m-postOverview__username" { (username) }
                    }

                    div ."m-postOverview__content" {
                        p {
                            (content)
                        }

                        p {
                            "Molestias commodi voluptate qui nemo veniam quis. Commodi rem quis sapiente omnis dolorem nihil qui. Name totam quaerat qui blanditiis et et enim."
                        }

                        p {
                            "Molestias commodi voluptate qui nemo veniam quis. Commodi rem quis sapiente omnis dolorem nihil qui. Name totam quaerat qui blanditiis et et enim."
                        }

                    }
                }
            }

            div ."m-postOverview__buttonBar"{
                // "Buttons here"
            }
        }
    }
}
