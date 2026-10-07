import { useId, useState } from "react";
import "./ManagerWorkspace.css";

type Choice = { id: string; label: string; searchText?: string; disabled?: boolean };

/** Keeps large native pickers searchable without replacing their keyboard behavior. */
export function ManagerSearchSelect({
  label,
  ariaLabel = label,
  emptyLabel,
  choices,
  value,
  disabled,
  onChange,
}: {
  label: string;
  ariaLabel?: string;
  emptyLabel?: string;
  choices: Choice[];
  value: string;
  disabled?: boolean;
  onChange: (id: string) => void;
}) {
  const id = useId();
  const [search, setSearch] = useState("");
  const [limit, setLimit] = useState(20);
  const query = search.trim().toLocaleLowerCase();
  const matching = choices.filter((choice) =>
    `${choice.label} ${choice.searchText ?? ""}`.toLocaleLowerCase().includes(query),
  );
  const visible = matching.slice(0, limit);
  const selected = choices.find((choice) => choice.id === value);
  // Filtering is only a way to find a choice, never a change to the saved selection or draft.
  const pinned = selected && !visible.some((choice) => choice.id === selected.id);
  return (
    <div className="manager-search-select">
      {(choices.length > 20 || search) && (
        <label>
          Search {label.toLocaleLowerCase()}
          <input
            type="search"
            aria-label={`Search ${ariaLabel}`}
            disabled={disabled}
            value={search}
            onChange={(event) => {
              setSearch(event.currentTarget.value);
              setLimit(20);
            }}
          />
        </label>
      )}
      <label htmlFor={id}>{label}</label>
      <select
        id={id}
        aria-label={ariaLabel}
        disabled={disabled}
        value={value}
        onChange={(event) => onChange(event.currentTarget.value)}
      >
        {emptyLabel !== undefined && <option value="">{emptyLabel}</option>}
        {pinned && (
          <option value={selected.id} disabled={selected.disabled}>
            {selected.label}
          </option>
        )}
        {visible.map((choice) => (
          <option key={choice.id} value={choice.id} disabled={choice.disabled}>
            {choice.label}
          </option>
        ))}
      </select>
      {(choices.length > 20 || search) && (
        <div className="manager-list-footer">
          <small>
            {matching.length
              ? `${Math.min(limit, matching.length)} of ${matching.length} matches`
              : "No matches. Try another search."}
            {pinned ? " · Current selection kept" : ""}
          </small>
          {matching.length > limit && (
            <button
              className="quiet-button"
              disabled={disabled}
              onClick={() => setLimit(limit + 20)}
            >
              More {label.toLocaleLowerCase()}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
