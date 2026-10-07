import { useId, useState } from "react";
import { Dialog } from "./Dialog";
import {
  navigationKey,
  navigationKindLabel,
  type NavigationCommand,
  type NavigationRecord,
  type NavigationSnapshot,
} from "./workspaceNavigationTypes";
import "./WorkspaceRecords.css";

type OpenRecord = (record: NavigationRecord) => void;

function RecordButton({
  record,
  disabled,
  onOpen,
}: {
  record: NavigationRecord;
  disabled: boolean;
  onOpen: OpenRecord;
}) {
  return (
    <button
      className="sidebar-destination workspace-record"
      disabled={disabled || record.workspaceState === "missing"}
      onClick={() => onOpen(record)}
      title={record.label}
      data-navigation-focus={`workspace-record:${navigationKey(record)}`}
    >
      <span className="workspace-record-label">{record.label}</span>
      <small>
        {navigationKindLabel(record)}
        {record.workspaceState !== "active" ? ` · ${record.workspaceState}` : ""}
      </small>
    </button>
  );
}

function RecordList({
  title,
  records,
  pageSize,
  disabled,
  onOpen,
  onUnpin,
  pending = false,
}: {
  title: string;
  records: NavigationRecord[];
  pageSize: number;
  disabled: boolean;
  onOpen: OpenRecord;
  onUnpin?: (record: NavigationRecord) => void;
  pending?: boolean;
}) {
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const matching = records.filter((record) =>
    `${record.label} ${navigationKindLabel(record)} ${record.workspaceState}`
      .toLocaleLowerCase()
      .includes(query.trim().toLocaleLowerCase()),
  );
  const currentPage = Math.min(page, Math.max(0, Math.ceil(matching.length / pageSize) - 1));
  return (
    <>
      {(records.length > pageSize || query) && (
        <label className="sidebar-search">
          <span className="sr-only">Find {title.toLowerCase()} records</span>
          <input
            type="search"
            value={query}
            placeholder={`Find ${title.toLowerCase()}…`}
            onChange={(event) => {
              setQuery(event.target.value);
              setPage(0);
            }}
          />
        </label>
      )}
      <ul className="workspace-record-list" aria-label={`${title} records`}>
        {matching.slice(currentPage * pageSize, (currentPage + 1) * pageSize).map((record) => (
          <li key={navigationKey(record)}>
            <RecordButton record={record} disabled={disabled} onOpen={onOpen} />
            {onUnpin && (
              <button
                className="quiet-button"
                disabled={pending}
                aria-label={`Unpin ${record.label}`}
                onClick={() => onUnpin(record)}
              >
                Unpin
              </button>
            )}
          </li>
        ))}
      </ul>
      {!matching.length && (
        <p className="muted sidebar-empty">
          {query
            ? "No matching records."
            : title === "Pinned"
              ? "Pin an open record from View."
              : "Opened records appear here."}
        </p>
      )}
      {matching.length > pageSize && (
        <nav className="workspace-record-pages" aria-label={`${title} record pages`}>
          <button
            className="quiet-button"
            aria-label={`Previous ${title.toLowerCase()} records`}
            disabled={currentPage === 0}
            onClick={() => setPage(currentPage - 1)}
          >
            ←
          </button>
          <small>
            {currentPage + 1} / {Math.ceil(matching.length / pageSize)}
          </small>
          <button
            className="quiet-button"
            aria-label={`Next ${title.toLowerCase()} records`}
            disabled={(currentPage + 1) * pageSize >= matching.length}
            onClick={() => setPage(currentPage + 1)}
          >
            →
          </button>
        </nav>
      )}
    </>
  );
}

function SidebarSection({
  title,
  records,
  disabled,
  onOpen,
}: {
  title: string;
  records: NavigationRecord[];
  disabled: boolean;
  onOpen: OpenRecord;
}) {
  const id = useId();
  const [open, setOpen] = useState(true);
  return (
    <section className="workspace-record-section">
      <button
        className="sidebar-section-toggle"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen(!open)}
      >
        {title} <small>{records.length}</small>
        <span aria-hidden="true">{open ? "▾" : "▸"}</span>
      </button>
      <div id={id} hidden={!open}>
        <RecordList
          title={title}
          records={records}
          pageSize={5}
          disabled={disabled}
          onOpen={onOpen}
        />
      </div>
    </section>
  );
}

export function WorkspaceRecordsSidebar({
  snapshot,
  error,
  disabled,
  pending,
  onOpen,
  onRetry,
  onManage,
}: {
  snapshot: NavigationSnapshot | null;
  error: string | null;
  disabled: boolean;
  pending: boolean;
  onOpen: OpenRecord;
  onRetry: () => void;
  onManage: () => void;
}) {
  return (
    <div className="workspace-records">
      {error && (
        <div role="status" className="sidebar-empty">
          <p>Pinned and recent records need attention.</p>
          <button className="quiet-button" disabled={pending} onClick={onRetry}>
            Retry shortcuts
          </button>
        </div>
      )}
      <SidebarSection
        title="Pinned"
        records={snapshot?.pins ?? []}
        disabled={disabled}
        onOpen={onOpen}
      />
      <SidebarSection
        title="Recent"
        records={snapshot?.recents ?? []}
        disabled={disabled}
        onOpen={onOpen}
      />
      <button className="quiet-button workspace-record-manage" onClick={onManage}>
        Manage shortcuts…
      </button>
    </div>
  );
}

export function WorkspaceRecordsDialog({
  open,
  onClose,
  snapshot,
  error,
  pending,
  disabled,
  onOpen,
  onApply,
  onRetry,
}: {
  open: boolean;
  onClose: () => void;
  snapshot: NavigationSnapshot | null;
  error: string | null;
  pending: boolean;
  disabled: boolean;
  onOpen: OpenRecord;
  onApply: (command: NavigationCommand) => void;
  onRetry: () => void;
}) {
  return (
    <Dialog open={open} title="Pinned and recent records" onClose={onClose}>
      <p className="muted">
        Shortcuts belong to this Project. Unpinning or clearing this list keeps your writing.
      </p>
      {error && (
        <div role="status">
          <p>{error}</p>
          <button disabled={pending} onClick={onRetry}>
            Retry shortcuts
          </button>
        </div>
      )}
      {!snapshot && !error && <p role="status">Loading shortcuts…</p>}
      {snapshot && (
        <>
          <h3>Pinned</h3>
          <RecordList
            title="Pinned"
            records={snapshot.pins}
            pageSize={10}
            disabled={disabled}
            onOpen={onOpen}
            pending={pending}
            onUnpin={(target) =>
              onApply({
                kind: "pin",
                target: { recordKind: target.recordKind, recordId: target.recordId },
                pinned: false,
              })
            }
          />
          <h3>Recent</h3>
          <div className="workspace-record-settings">
            <label>
              Keep recent records
              <select
                value={snapshot.recentLimit}
                disabled={pending}
                onChange={(event) =>
                  onApply({ kind: "set_recent_limit", limit: Number(event.target.value) })
                }
              >
                {[...new Set([10, 20, 50, snapshot.recentLimit])]
                  .sort((a, b) => a - b)
                  .map((limit) => (
                    <option key={limit} value={limit}>
                      {limit}
                    </option>
                  ))}
              </select>
            </label>
            <button
              className="quiet-button"
              disabled={pending || !snapshot.recents.length}
              onClick={() => onApply({ kind: "clear_recents" })}
            >
              Clear recent records
            </button>
          </div>
          <RecordList
            title="Recent"
            records={snapshot.recents}
            pageSize={10}
            disabled={disabled}
            onOpen={onOpen}
          />
        </>
      )}
    </Dialog>
  );
}
