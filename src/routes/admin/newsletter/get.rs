use axum::response::Html;
use axum_flash::IncomingFlashes;
use std::fmt::Write;

pub async fn publish_newsletter_form(flashes: IncomingFlashes) -> Html<String> {
    let mut msg_html = String::new();
    for (_, msg) in flashes.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", msg).unwrap();
    }
    let idempotency_key = uuid::Uuid::new_v4();
    Html(format!(
        r#"<!DOCTYPE html>
    <html lang="en">
    
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Create newsletter</title>
    </head>
    
    <body>
        {msg_html}
        <form action="/admin/newsletter" method="post">
            <label>Newsletter title
                <input type="text" name="title" required>
            </label>
            <br>
            <label>HTML
                <input type="text" name="html" required>
            </label>
            <br>
            <label>Plain Text
                <input type="text" name="text" required>
            </label>
            <input type="text" hidden name="idempotency_key" value="{idempotency_key}"
            <br>
            <button type="submit">Create Newsletter</button>
        </form>
        <p><a href="/admin/dashboard">&lt;- Back</a></p>
    </body>
    </html>"#,
    ))
}
