export const Architecture = () => (
  <figure
    className="architecture-diagram not-prose"
    role="img"
    aria-label="ParadeDB architecture. Your application connects to Postgres through SQL. Inside Postgres, your tables are indexed by the pg_search extension, with index updates in the same transaction as table writes. One ParadeDB Index combines an inverted index for text, a clustered index for vectors, and columnar storage for filters and aggregates."
  >
    <div aria-hidden="true">
      <div className="architecture-app">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
        >
          <rect x="3" y="4" width="18" height="16" />
          <path d="M3 9h18" opacity=".5" />
          <path d="M7 6.5h.01M10 6.5h.01M9 12l-3 2.5L9 17m6-5 3 2.5-3 2.5" />
        </svg>
        <span>Your application</span>
      </div>
      <div className="architecture-connection architecture-sql">
        <svg
          viewBox="0 0 16 48"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
        >
          <path d="M8 1v45M4 6l4-4 4 4M4 41l4 4 4-4" />
        </svg>
        <span>SQL</span>
      </div>
      <div className="architecture-postgres">
        <div className="architecture-database-heading">
          <div className="architecture-heading">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <ellipse cx="12" cy="5" rx="8" ry="3" />
              <path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" />
            </svg>
            <span>Postgres</span>
          </div>
        </div>
        <div className="architecture-table">
          <svg
            className="architecture-table-icon"
            viewBox="0 0 40 40"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <path
              d="M4 5h32v10H4Z"
              fill="currentColor"
              fillOpacity=".08"
              stroke="none"
            />
            <rect x="4" y="5" width="32" height="30" />
            <path d="M4 15h32M4 25h32M15 15v20" />
            <path d="M10 10h5m5 0h10M21 20h8M21 30h8" strokeLinecap="round" />
          </svg>
          <div>
            <div className="architecture-title">Your tables</div>
            <div className="architecture-detail">
              Application data · Transactional writes
            </div>
          </div>
        </div>
        <div className="architecture-connection architecture-writes">
          <svg
            viewBox="0 0 16 48"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <path d="M8 0v45M4 40l4 5 4-5" />
          </svg>
          <span>Indexed in the same transaction</span>
        </div>
        <div className="architecture-index">
          <div className="architecture-index-heading">
            <div className="architecture-heading">
              <svg
                className="architecture-brand"
                viewBox="6 25 53 39"
                fill="currentColor"
              >
                <path d="M38.4926 26.8779H29.229V61.7664H38.4926ZM27.8173 26.8779H18.5537V61.7664H27.8173ZM17.1415 26.8779H7.87793V61.7664H17.1415ZM39.9048 26.8809V49.4682C39.9048 52.7387 41.2031 55.8462 43.514 58.1578C45.825 60.4688 48.9331 61.7671 52.2037 61.7671H57.6185V52.5034H52.2037C51.3973 52.5034 50.6325 52.1769 50.0638 51.6081C49.495 51.0393 49.1684 50.2745 49.1684 49.4682V36.363C49.1684 31.2208 45.0198 26.9994 39.9048 26.8814Z" />
              </svg>
              <span>ParadeDB Index</span>
            </div>
            <span className="architecture-extension">pg_search</span>
          </div>
          <div className="architecture-structures">
            <div className="architecture-structure">
              <svg
                viewBox="0 0 64 40"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              >
                <rect
                  x="2"
                  y="4"
                  width="16"
                  height="8"
                  fill="currentColor"
                  fillOpacity=".2"
                />
                <rect x="2" y="17" width="16" height="8" />
                <rect x="2" y="30" width="16" height="8" />
                <path d="M18 8h12m-12 13h12M18 34h12" opacity=".45" />
                <rect
                  x="30"
                  y="4"
                  width="8"
                  height="8"
                  fill="currentColor"
                  fillOpacity=".12"
                />
                <rect
                  x="42"
                  y="4"
                  width="8"
                  height="8"
                  fill="currentColor"
                  fillOpacity=".12"
                />
                <rect
                  x="54"
                  y="4"
                  width="8"
                  height="8"
                  fill="currentColor"
                  fillOpacity=".12"
                />
                <rect x="30" y="17" width="8" height="8" />
                <rect x="42" y="17" width="8" height="8" />
                <rect x="30" y="30" width="8" height="8" />
              </svg>
              <div className="architecture-title">Text</div>
              <div className="architecture-detail">Inverted index</div>
            </div>
            <div className="architecture-structure">
              <svg
                viewBox="0 0 64 40"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              >
                <path
                  d="m13 29 14-17 13 16 13-19M27 12l26-3M13 29l27-1"
                  opacity=".45"
                />
                <circle
                  cx="13"
                  cy="29"
                  r="4"
                  fill="var(--architecture-index)"
                />
                <circle cx="27" cy="12" r="4" fill="currentColor" />
                <circle
                  cx="40"
                  cy="28"
                  r="4"
                  fill="var(--architecture-index)"
                />
                <circle cx="53" cy="9" r="4" fill="var(--architecture-index)" />
                <circle cx="27" cy="12" r="9" opacity=".2" />
              </svg>
              <div className="architecture-title">Vector</div>
              <div className="architecture-detail">Clustered index</div>
            </div>
            <div className="architecture-structure">
              <svg
                viewBox="0 0 64 40"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              >
                <rect x="7" y="3" width="12" height="35" />
                <rect
                  x="26"
                  y="3"
                  width="12"
                  height="35"
                  fill="currentColor"
                  fillOpacity=".2"
                />
                <rect x="45" y="3" width="12" height="35" />
                <path
                  d="M7 12h12M7 21h12M7 30h12M26 12h12M26 21h12M26 30h12M45 12h12M45 21h12M45 30h12"
                  opacity=".6"
                />
              </svg>
              <div className="architecture-title">Filters and aggregates</div>
              <div className="architecture-detail">Columnar</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </figure>
);
