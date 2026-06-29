Using a **Tailscale Funnel** for your Raspberry Pi changes things for the better. Tailscale automatically provisions an officially trusted Let's Encrypt SSL certificate for your public funnel domain.

This means your backend is *already* serving via `https://`, which completely satisfies the browser's security requirements.

Here is exactly how to link your GitHub Pages frontend to your Tailscale Funnel backend.

---

## 1. Direct the Client to the Funnel URL

In your `main.rs` layout, conditionally tell the WASM frontend to route all server functions directly to your Tailscale Funnel.

```rust
use dioxus::prelude::*;

fn main() {
    // Only configure this when building for the browser client
    #[cfg(feature = "web")]
    {
        dioxus::fullstack::set_server_url("https://toph.tail241b81.ts.net");
    }

    dioxus::launch(app);
}

```

## 2. Apply CORS Rules on the Axum/Dioxus Server

Even though the security protocol (`https`) now matches, the browser will still block your requests if it notices a cross-domain fetch (`github.io` asking `ts.net` for data). You must explicitly configure CORS on the backend.

If you are using a standard Axum router setup to launch your Dioxus Fullstack application, add `tower-http` to your backend dependencies:

```toml
[dependencies]
tower-http = { version = "0.6", features = ["cors"] }

```

Then attach the layer to your Axum app initialization block:

```rust
use axum::http::{header, Method};
use tower_http::cors::{Any, CorsLayer};

// Build your server config
let cors = CorsLayer::new()
    // Explicitly allow your GitHub pages domain
    .allow_origin([
        "https://<your-github-username>.github.io".parse().unwrap(),
        "http://localhost:8080".parse().unwrap(), // Keeps local dev functional
    ])
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([header::CONTENT_TYPE]);

// If building your Axum Router manually:
let app = Router::new()
    .nested_bootstrap_with_config(config)
    .layer(cors);

```

*(If you are using a more basic `LaunchBuilder`, you can pass this layer directly inside your server-specific initialization block using standard Axum middleware mapping.)*

## 3. Shipping to GitHub Pages

To avoid relative asset paths breaking if your GitHub Pages site sits on a repository sub-path (e.g., `[https://username.github.io/my-repo-name/](https://username.github.io/my-repo-name/)`), use the `--base-url` flag when bundling:

```bash
dx build --platform web --release --base-url "/<your-repo-name>/"

```

Take the contents of the generated `dist/` folder and push them directly to your repository's target deployment branch.
