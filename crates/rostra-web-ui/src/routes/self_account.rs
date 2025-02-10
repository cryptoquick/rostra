mod extractor;

use axum::extract::State;
use axum::response::IntoResponse;
use maud::{html, Markup, PreEscaped};
use rostra_client::ClientRef;
use rostra_client_db::IdSocialProfileRecord;
use rostra_core::id::{RostraId, ToShort as _};
use rostra_core::ShortEventId;

use super::unlock::session::{RoMode, UserSession};
use super::Maud;
use crate::error::RequestResult;
use crate::html_utils::submit_on_ctrl_enter;
use crate::{SharedState, UiState};

pub async fn get_self_account_edit(
    state: State<SharedState>,
    session: UserSession,
) -> RequestResult<impl IntoResponse> {
    Ok(Maud(state.render_profile_edit_form(&session).await?))
}

pub async fn post_self_account_edit(
    state: State<SharedState>,
    session: UserSession,
    form: extractor::InputForm,
) -> RequestResult<impl IntoResponse> {
    let existing = state
        .client(session.id())
        .await?
        .client_ref()?
        .db()
        .get_social_profile(session.id())
        .await;

    state
        .client(session.id())
        .await?
        .client_ref()?
        .post_social_profile_update(
            session.id_secret()?,
            form.name,
            form.bio,
            form.avatar.or_else(|| existing.and_then(|e| e.avatar)),
        )
        .await?;

    Ok(Maud(
        state
            .render_self_profile_summary(&session, session.ro_mode())
            .await?,
    ))
}

impl UiState {
    pub async fn get_social_profile(
        &self,
        id: RostraId,
        client: &ClientRef<'_>,
    ) -> RequestResult<IdSocialProfileRecord> {
        let existing = client.db().get_social_profile(id).await.unwrap_or_else(|| {
            rostra_client_db::IdSocialProfileRecord {
                event_id: ShortEventId::ZERO,
                display_name: id.to_short().to_string(),
                bio: "".into(),
                avatar: None,
            }
        });
        Ok(existing)
    }

    pub fn avatar_url(&self, id: RostraId) -> String {
        format!("/ui/avatar/{}", id)
    }

    pub async fn render_self_profile_summary(
        &self,
        user: &UserSession,
        ro: RoMode,
    ) -> RequestResult<Markup> {
        let client = self.client(user.id()).await?;
        let self_id = client.client_ref()?.rostra_id();
        let self_profile = self
            .get_social_profile(self_id, &client.client_ref()?)
            .await?;
        Ok(html! {
            div ."m-selfAccount" {
                script {
                    (PreEscaped(
                    r#"
                    console.log("THERE");
                    function copyIdToClipboard(event) {
                    console.log("HERE");
                        const target = event.target;
                        const id = target.getAttribute('data-value');
                        navigator.clipboard.writeText(id);
                        target.classList.add('-active');

                        setTimeout(() => {
                            target.classList.remove('-active');
                        }, 1000);
                    }
                    "#
                    ))
                }
                img ."m-selfAccount__userImage u-userImage"
                    src=(self.avatar_url(self_id))
                    width="32pt"
                    height="32pt"
                    { }

                div ."m-selfAccount__content" {
                    span ."m-selfAccount__displayName" { (self_profile.display_name) }
                    div ."m-selfAccount__buttons" {
                        button
                            ."m-selfAccount__copyButton u-button"
                            data-value=(self.client(user.id()).await?.client_ref()?.rostra_id()) onclick="copyIdToClipboard(event)"  {
                                span ."m-selfAccount__copyButtonIcon u-buttonIcon" width="1rem" height="1rem" {}
                                "RostraId"
                            }
                        button
                            ."m-selfAccount__editButton u-button"
                            hx-get="/ui/self/edit"
                            hx-target="closest .m-selfAccount"
                            hx-swap="outerHTML"
                            disabled[ro.to_disabled()]
                            {
                                span ."m-selfAccount__editButtonIcon u-buttonIcon" width="1rem" height="1rem" {}
                                "Edit"
                            }
                        button
                            ."m-selfAccount__logoutButton u-button"
                            hx-get="/ui/unlock/logout"
                            {
                                span ."m-selfAccount__logoutButtonIcon u-buttonIcon" width="1rem" height="1rem" {}
                                "Logout"
                            }
                    }
                }
            }
        })
    }

    pub async fn render_profile_edit_form(&self, user: &UserSession) -> RequestResult<Markup> {
        let client = self.client(user.id()).await?;
        let client_ref = client.client_ref()?;
        let self_profile = self
            .get_social_profile(client_ref.rostra_id(), &client_ref)
            .await?;
        Ok(html! {
            form ."m-selfAccount -edit"
                hx-post="/ui/self/edit"
                hx-swap="outerHTML"
                hx-encoding="multipart/form-data"
            {
                script {
                    (PreEscaped(r#"
                        function previewAvatar(event) {
                            document.querySelector('.m-selfAccount__userImage').src=URL.createObjectURL(event.target.files[0]);
                        }    
                    "#))
                }
                label for="avatar-upload" ."m-selfAccount__userImageLabel" {
                    img ."m-selfAccount__userImage"
                        src=(self.avatar_url(user.id()))
                        width="32pt"
                        height="32pt" {
                    }
                }
                input #"avatar-upload"
                    ."m-selfAccount__userImageInput"
                    type="file"
                    name="avatar"
                    accept="image/*"
                    style="display: none;"
                    onchange="previewAvatar(event)"
                {}

                div ."m-selfAccount__content" {
                    input ."m-selfAccount__displayName"
                        type="text"
                        name="name"
                        value=(self_profile.display_name) {
                    }
                    textarea."m-selfAccount__bio"
                        placeholder="Bio..."
                        type="text"
                        dir="auto"
                        name="bio" {
                        {(self_profile.bio)}
                    }

                    div ."m-selfAccount__buttons" {
                        button
                            ."m-selfAccount__saveButton u-button" {
                            span ."m-selfAccount__saveButtonIcon u-buttonIcon" width="1rem" height="1rem" {}
                            "Save"
                        }
                    }
                }
            }
            (submit_on_ctrl_enter(".m-selfAccount", ".m-selfAccount__bio"))
        })
    }
}
