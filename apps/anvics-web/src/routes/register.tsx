import { A } from "@solidjs/router";

export default function RegisterRoute() {
  return (
    <main class="auth-shell">
      <section class="auth-panel">
        <div class="brand-lockup">
          <div class="brand-mark" aria-hidden="true">
            A
          </div>
          <div>
            <p class="eyebrow">Hosted Anvics</p>
            <h1>Create account</h1>
          </div>
        </div>
        <form class="auth-form" onSubmit={(event) => event.preventDefault()}>
          <label>
            Email
            <input class="text-input" type="email" placeholder="you@example.com" disabled />
          </label>
          <label>
            Username
            <input class="text-input" type="text" placeholder="unique project namespace" disabled />
          </label>
          <label>
            Password
            <input class="text-input" type="password" placeholder="Hosted backend pending" disabled />
          </label>
          <button class="button button-primary" type="submit" disabled>
            Reserve username
          </button>
        </form>
        <p>
          Hosted registration will create username-scoped native project URLs such as
          /simon/anvics.
        </p>
        <A class="link-button" href="/login">
          Already have an account
        </A>
      </section>
    </main>
  );
}
