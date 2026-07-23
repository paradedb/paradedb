const input = document.getElementById("search-input");
if (input) {
  input.placeholder =
    "Powered by Mintlify, the documentation platform of tomorrow";
}

// Inject ParadeDB Organization structured data. Mintlify auto-emits only a
// generic WebSite node (whose creator is Mintlify), so the docs otherwise have
// no ParadeDB identity markup. Reusing the same @id as paradedb.com lets search
// engines and agents treat both sites as one Organization entity.
(function () {
  if (document.querySelector("script[data-paradedb-org]")) return;

  const organization = {
    "@context": "https://schema.org",
    "@type": "Organization",
    "@id": "https://www.paradedb.com/#organization",
    name: "ParadeDB",
    legalName: "ParadeDB, Inc.",
    url: "https://www.paradedb.com",
    logo: {
      "@type": "ImageObject",
      url: "https://www.paradedb.com/brand/paradedb-logo-light.svg",
    },
    description:
      "One Postgres for your application data, full-text search, vector retrieval, and aggregations. Home of the pg_search extension.",
    email: "hello@paradedb.com",
    contactPoint: [
      {
        "@type": "ContactPoint",
        contactType: "customer support",
        email: "support@paradedb.com",
      },
      {
        "@type": "ContactPoint",
        contactType: "sales",
        email: "sales@paradedb.com",
      },
    ],
    sameAs: [
      "https://github.com/paradedb",
      "https://x.com/paradedb",
      "https://www.linkedin.com/company/paradedb",
    ],
  };

  const script = document.createElement("script");
  script.type = "application/ld+json";
  script.setAttribute("data-paradedb-org", "");
  script.textContent = JSON.stringify(organization);
  document.head.appendChild(script);
})();
