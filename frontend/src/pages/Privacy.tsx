import { Link } from "react-router";

import { Card } from "../components/ui";
import { t } from "../i18n/fr";

/**
 * Public privacy policy (required by Apple for Sign in with Apple / the App
 * Store, and by Google for a published OAuth app). Static French copy;
 * the `[à compléter]` markers must be filled by the operator before the
 * OAuth apps are published.
 */
export default function Privacy() {
  const p = t.privacy;
  return (
    <main className="mx-auto max-w-2xl px-4 py-8">
      <h1 className="text-2xl font-bold text-abysse-700">{p.title}</h1>
      <p className="mt-1 text-sm text-ink-soft">{p.updated}</p>
      <Card className="mt-6">
        <div className="flex flex-col gap-5 text-sm leading-relaxed">
          {p.sections.map((section) => (
            <section key={section.title}>
              <h2 className="font-semibold">{section.title}</h2>
              {section.paragraphs.map((text) => (
                <p key={text} className="mt-1 text-ink-soft">
                  {text}
                </p>
              ))}
            </section>
          ))}
        </div>
      </Card>
      <Link to="/login" className="mt-6 inline-block text-sm font-semibold text-abysse-700">
        {t.magicLink.backToLogin}
      </Link>
    </main>
  );
}
