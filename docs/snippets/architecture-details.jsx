export const ArchitectureDiagram = () => {
  const IndexMark = ({ x, y }) => (
    <svg
      x={x}
      y={y}
      width="18"
      height="18"
      viewBox="6 25 53 39"
      fill="var(--architecture-accent)"
    >
      <path d="M38.4926 26.8779H29.229V61.7664H38.4926ZM27.8173 26.8779H18.5537V61.7664H27.8173ZM17.1415 26.8779H7.87793V61.7664H17.1415ZM39.9048 26.8809V49.4682C39.9048 52.7387 41.2031 55.8462 43.514 58.1578C45.825 60.4688 48.9331 61.7671 52.2037 61.7671H57.6185V52.5034H52.2037C51.3973 52.5034 50.6325 52.1769 50.0638 51.6081C49.495 51.0393 49.1684 50.2745 49.1684 49.4682V36.363C49.1684 31.2208 45.0198 26.9994 39.9048 26.8814Z" />
    </svg>
  );

  const Arrow = ({ x, y, direction = "down" }) => (
    <g
      transform={`translate(${x} ${y}) rotate(${direction === "right" ? -90 : direction === "left" ? 90 : direction === "up" ? 180 : 0})`}
    >
      <path className="architecture-svg-line" d="M-5-6 0 0 5-6" />
    </g>
  );

  const Segment = ({ number, compact = false }) => (
    <g>
      <rect
        className="architecture-svg-surface"
        width={compact ? 80 : 152}
        height={compact ? 64 : 48}
      />
      {compact ? (
        <g>
          <text
            className="architecture-svg-small"
            x="40"
            y="25"
            textAnchor="middle"
          >
            Segment
          </text>
          <text x="40" y="49" textAnchor="middle">
            {number}
          </text>
        </g>
      ) : (
        <text x="76" y="30" textAnchor="middle">
          Segment {number}
        </text>
      )}
    </g>
  );

  const DataModelSvg = ({ compact = false }) => (
    <svg
      className={`architecture-svg architecture-svg-${compact ? "compact" : "wide"}`}
      viewBox={compact ? "0 0 320 536" : "0 0 640 354"}
      aria-hidden="true"
    >
      <rect
        className="architecture-svg-boundary"
        x="0.75"
        y="0.75"
        width={compact ? 318.5 : 638.5}
        height={compact ? 534.5 : 352.5}
      />
      <IndexMark x={24} y={19} />
      <text className="architecture-svg-title" x="52" y="34">
        ParadeDB Index
      </text>
      {!compact && (
        <text
          className="architecture-svg-label"
          x="616"
          y="34"
          textAnchor="end"
        >
          LSM tree
        </text>
      )}
      <text
        className="architecture-svg-label"
        x={compact ? 160 : 100}
        y={compact ? 78 : 86}
        textAnchor="middle"
      >
        Incoming writes
      </text>
      <path
        className="architecture-svg-line"
        d={compact ? "M160 90V112" : "M100 98V154"}
      />
      <Arrow x={compact ? 160 : 100} y={compact ? 112 : 154} />
      <g transform={`translate(24 ${compact ? 112 : 154})`}>
        <rect
          className="architecture-svg-surface"
          width={compact ? 272 : 152}
          height={compact ? 76 : 88}
        />
        <text
          className="architecture-svg-stage-title"
          x="16"
          y={compact ? 30 : 35}
        >
          Write buffer
        </text>
        <text className="architecture-svg-label" x="16" y={compact ? 54 : 59}>
          Mutable segment
        </text>
      </g>
      <path
        className="architecture-svg-line"
        d={
          compact
            ? "M160 188V222M64 240V222H256V240M160 222V240"
            : "M176 198H230M252 134H230V262H252M230 198H252"
        }
      />
      <text
        className="architecture-svg-label"
        x={compact ? 178 : 204}
        y={compact ? 212 : 184}
        textAnchor={compact ? "start" : "middle"}
      >
        Flush
      </text>
      {[1, 2, 3].map((number, i) => (
        <g key={number}>
          <Arrow
            x={compact ? 64 + i * 96 : 252}
            y={compact ? 240 : 134 + i * 64}
            direction={compact ? "down" : "right"}
          />
          <g
            transform={`translate(${compact ? 24 + i * 96 : 252} ${compact ? 240 : 110 + i * 64})`}
          >
            <Segment number={number} compact={compact} />
          </g>
        </g>
      ))}
      {!compact && (
        <text
          className="architecture-svg-label"
          x="328"
          y="92"
          textAnchor="middle"
        >
          Immutable segments
        </text>
      )}
      <path
        className="architecture-svg-line"
        d={
          compact
            ? "M64 304V330H256V304M160 304V372"
            : "M404 134H418V262H404M404 198H476"
        }
      />
      <Arrow
        x={compact ? 160 : 476}
        y={compact ? 372 : 198}
        direction={compact ? "down" : "right"}
      />
      <text
        className="architecture-svg-label"
        x={compact ? 178 : 448}
        y={compact ? 358 : 184}
        textAnchor={compact ? "start" : "middle"}
      >
        Merge
      </text>
      <g transform={`translate(${compact ? 24 : 476} ${compact ? 372 : 144})`}>
        <rect
          className="architecture-svg-index"
          width={compact ? 272 : 140}
          height={compact ? 80 : 108}
        />
        {compact ? (
          <g>
            <text className="architecture-svg-stage-title" x="16" y="32">
              Merged segment
            </text>
            <text className="architecture-svg-label" x="16" y="56">
              Larger, immutable segment
            </text>
          </g>
        ) : (
          <g>
            <text className="architecture-svg-stage-title" x="16" y="33">
              Merged
            </text>
            <text className="architecture-svg-stage-title" x="16" y="56">
              segment
            </text>
            <text className="architecture-svg-label" x="16" y="84">
              Immutable
            </text>
          </g>
        )}
      </g>
      <path
        className="architecture-svg-rule"
        d={compact ? "M24 474H296" : "M24 308H616"}
      />
      {compact ? (
        <text
          className="architecture-svg-label"
          x="160"
          y="498"
          textAnchor="middle"
        >
          <tspan x="160">Each segment contains</tspan>
          <tspan x="160" dy="20">
            text, vector, and columnar structures.
          </tspan>
        </text>
      ) : (
        <text className="architecture-svg-label" x="24" y="333">
          Each segment contains text, vector, and columnar structures.
        </text>
      )}
    </svg>
  );

  const DataModelDiagram = () => (
    <figure
      className="architecture-diagram architecture-detail-diagram not-prose"
      role="img"
      aria-label="ParadeDB Index data model. Incoming writes enter a mutable write buffer. Flushing creates immutable segments. Merging combines smaller segments into a larger immutable segment. Each segment contains text, vector, and columnar structures."
    >
      <DataModelSvg />
      <DataModelSvg compact />
    </figure>
  );

  return <DataModelDiagram />;
};
