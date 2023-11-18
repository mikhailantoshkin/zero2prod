use axum::response::{Html, IntoResponse, Response};
use axum_flash::IncomingFlashes;

pub async fn login_form(flashes: IncomingFlashes) -> Response {
    let error_html: String = match flashes.into_iter().next() {
        None => "".into(),
        Some((_, text)) => format!("<p><i>{}</i></p>", text),
    };

    let resp = Html(format!(
        r#"
<!DOCTYPE html>
<html lang="en">

<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>

<body>
    {error_html}
    <form action="/login" method="post">
        <label>Username
            <input type="text" placeholder="Enter Username" name="username">
        </label>
        <label>Password
            <input type="password" placeholder="Enter Password" name="password">
        </label>
        <button type="submit">Login</button>
    </form>
</body>

</html>"#,
    ));
    (flashes, resp).into_response()
}
