import { useState } from "react";
import {
  IdentityProvider,
  Products,
  StytchPasskeyRegistration,
  useStytch,
  useStytchUser,
} from "@stytch/react";
import { ApprovalSetup, Approvals } from "./approvals.jsx";
import { SignIn, authPresentation } from "./auth.jsx";
import { WorkDashboard } from "./work-dashboard.jsx";
import { Icon, LoadingState, Notice, PageHeader } from "./ui.jsx";

function Authorize() {
  const { user, isInitialized } = useStytchUser();
  if (!isInitialized) return <LoadingState />;
  if (!user)
    return (
      <SignIn purpose="Sign in to review access for the requesting app." />
    );
  return (
    <div className="narrow-page">
      <PageHeader
        eyebrow="APP CONNECTION"
        title="Make the connection."
        description="Review the access requested by your app before continuing."
      />
      <section className="panel stytch-panel">
        <IdentityProvider
          presentation={{
            ...authPresentation,
            options: { hideHeaderText: false },
          }}
        />
      </section>
    </div>
  );
}

function Passkeys() {
  const { user, isInitialized } = useStytchUser();
  if (!isInitialized) return <LoadingState />;
  if (!user)
    return <SignIn purpose="Sign in to manage your sign-in passkeys." />;
  return (
    <div className="narrow-page">
      <a className="back-link" href="/approve/setup">
        ← Back to security
      </a>
      <PageHeader
        eyebrow="ACCOUNT SECURITY"
        title="Sign-in passkeys"
        description="A familiar, secure way back into your workspace."
      />
      <section className="panel passkey-panel">
        <span className="feature-icon">
          <Icon name="key" size={25} />
        </span>
        <h2>Add a sign-in passkey</h2>
        <p className="muted">
          Save a passkey to your device or password manager. Use it the next
          time you sign in.
        </p>
        <div className="stytch-widget">
          <StytchPasskeyRegistration
            config={{ products: [Products.passkeyRegistration] }}
            presentation={authPresentation}
          />
        </div>
      </section>
      <p className="footnote">
        <Icon name="shield" size={16} />
        This passkey gives you access to your account. Approval passkeys sign
        your decisions.
      </p>
    </div>
  );
}

function Home() {
  const { user, isInitialized } = useStytchUser();
  if (!isInitialized) return <LoadingState />;
  if (!user) return <SignIn />;
  const email = user.emails?.[0]?.email ?? user.user_id;
  const cards = [
    { href: "/work", icon: "clock", label: "Your work", title: "View work", text: "Follow reported task observations and usage.", link: "Open work" },
    {
      href: "/approve",
      icon: "inbox",
      label: "Your decisions",
      title: "Review approvals",
      text: "See what needs your attention and sign your answer.",
      featured: true,
    },
    {
      href: "/approve/setup",
      icon: "shield",
      label: "Your signature",
      title: "Approval passkeys",
      text: "Manage the keys you use to sign your decisions.",
    },
    {
      href: "/passkeys",
      icon: "key",
      label: "Your access",
      title: "Sign-in passkeys",
      text: "Add a passkey for a smoother way back in.",
    },
  ];
  return (
    <>
      <PageHeader
        eyebrow="YOUR WORKSPACE"
        title="Welcome back."
        description="Your access, your approvals. All in one place."
      />
      <section className="account-banner">
        <span className="account-avatar">
          {email.slice(0, 1).toUpperCase()}
        </span>
        <div className="account-identity">
          <span className="eyebrow">SIGNED IN AS</span>
          <strong>{email}</strong>
        </div>
        <span className="badge badge-green">
          <Icon name="check" size={14} />
          Signed in
        </span>
      </section>
      <div className="action-grid">
        {cards.map((card) => (
          <a
            className={`action-card${card.featured ? " action-card-featured" : ""}`}
            href={card.href}
            key={card.href}
          >
            <span className="feature-icon">
              <Icon name={card.icon} size={25} />
            </span>
            <span className="eyebrow">{card.label}</span>
            <h2>{card.title}</h2>
            <p>{card.text}</p>
            <span className="card-link">
              {card.link || (card.featured ? "Open approvals" : "Manage passkeys")}
              <Icon name="arrow" size={18} />
            </span>
          </a>
        ))}
      </div>
      <aside className="workspace-note">
        <Icon name="shield" size={22} />
        <div>
          <strong>Access and approval stay separate.</strong>
          <p>
            Signing in opens your workspace. Each approval is a separate
            decision, signed with your approval passkey.
          </p>
        </div>
      </aside>
      <details className="account-details">
        <summary>Account details</summary>
        <p className="muted">
          User ID <code>{user.user_id}</code>
        </p>
      </details>
    </>
  );
}

const routes = {
  "/": Home,
  "/oauth/authorize": Authorize,
  "/passkeys": Passkeys,
  "/approve": Approvals,
  "/work": WorkDashboard,
  "/approve/setup": ApprovalSetup,
};

export default function App() {
  const stytch = useStytch();
  const { user } = useStytchUser();
  const [signingOut, setSigningOut] = useState(false);
  const [error, setError] = useState("");
  const path = window.location.pathname.replace(/\/+$/, "") || "/";
  const Page = routes[path];
  const email = user?.emails?.[0]?.email;
  const nav = [
    { href: "/", text: "Overview", active: path === "/" },
    { href: "/work", text: "Work", active: path === "/work" },
    { href: "/approve", text: "Approvals", active: path === "/approve" },
    {
      href: "/approve/setup",
      text: "Security",
      active: ["/approve/setup", "/passkeys"].includes(path),
    },
  ];
  const signOut = async () => {
    setSigningOut(true);
    setError("");
    try {
      await stytch.session.revoke();
    } catch {
      setError("Could not sign out. Please try again.");
    } finally {
      setSigningOut(false);
    }
  };
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <header className="site-header">
        <div className="header-inner">
          <a className="brand" href="/" aria-label="devwits home">
            <span className="brand-mark" aria-hidden="true">
              d<span>·</span>
            </span>
            devwits
            <span className="brand-divider" />
            <span className="brand-product">workspace</span>
          </a>
          {user && (
            <nav aria-label="Main navigation">
              {nav.map((item) => (
                <a
                  href={item.href}
                  key={item.href}
                  aria-current={item.active ? "page" : undefined}
                >
                  {item.text}
                </a>
              ))}
            </nav>
          )}
          <div className="header-account">
            {user ? (
              <>
                <span
                  className="header-avatar"
                  title={email || "Your account"}
                  aria-hidden="true"
                >
                  {(email || "Y").slice(0, 1).toUpperCase()}
                </span>
                <button
                  className="button button-quiet sign-out"
                  type="button"
                  onClick={signOut}
                  disabled={signingOut}
                >
                  <Icon name="logout" size={16} />
                  {signingOut ? "Signing out…" : "Sign out"}
                </button>
              </>
            ) : (
              <span className="header-secure">
                <Icon name="lock" size={14} />
                Personal workspace
              </span>
            )}
          </div>
        </div>
      </header>
      <main
        id="main"
        className={`main-content${!user && Page ? " main-auth" : ""}`}
      >
        {error && <Notice tone="error">{error}</Notice>}
        {Page ? (
          <Page />
        ) : (
          <div className="not-found">
            <span className="eyebrow">404 · PAGE NOT FOUND</span>
            <h1>A little off the path.</h1>
            <p className="muted">
              This page doesn’t exist. Your workspace is right here.
            </p>
            <a className="button button-primary" href="/">
              Back to your workspace
              <Icon name="arrow" size={18} />
            </a>
          </div>
        )}
      </main>
      <footer className="site-footer">
        <span>
          devwits<span className="footer-dot">·</span>A little more considered.
        </span>
        <span>
          <Icon name="lock" size={13} />
          Your workspace. Your decisions.
        </span>
      </footer>
    </div>
  );
}
