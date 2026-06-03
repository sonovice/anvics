import { A } from "@solidjs/router";

export default function LoginRoute() {
  return (
    <main class="auth-shell">
      <section class="auth-panel">
        <div class="brand-lockup">
          <div class="brand-mark" aria-hidden="true">
            A
          </div>
          <div>
            <p class="eyebrow">Hosted Anvics</p>
            <h1>Sign in</h1>
          </div>
        </div>
        <form class="auth-form" onSubmit={(event) => event.preventDefault()}>
          <label>
            Email
            <input class="text-input" type="email" placeholder="you@example.com" disabled />
          </label>
          <label>
            Password
            <input class="text-input" type="password" placeholder="Hosted backend pending" disabled />
          </label>
          <button class="button button-primary" type="submit" disabled>
            Sign in
          </button>
        </form>
        <p>
          Hosted identity is reserved for the native sync milestone. Local Anvics does
          not need an account.
        </p>
        <A class="link-button" href="/register">
          Create hosted account
        </A>
      </section>
    </main>
  );
}
