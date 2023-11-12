use axum::response::{Html, IntoResponse, Response};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};

pub async fn login_form(mut jar: SignedCookieJar) -> Response {
    let error_html: String = match jar.get("_flash") {
        None => "".into(),
        Some(cookie) => {
            jar = jar.remove(Cookie::named("_flash"));
            format!("<p><i>{}</i></p>", cookie.value())
        }
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
    (jar, resp).into_response()
}
