use axum::response::Html;

pub async fn publish_newsletter_form() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
    <html lang="en">
    
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Create newsletter</title>
    </head>
    
    <body>
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
            <br>
            <button type="submit">Create Newsletter</button>
        </form>
        <p><a href="/admin/dashboard">&lt;- Back</a></p>
    </body>
    </html>"#,
    )
}
