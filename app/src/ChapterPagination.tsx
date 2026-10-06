export function ChapterPagination({
  page,
  pageSize,
  total,
  label,
  onPage,
  disabled = false,
}: {
  page: number;
  pageSize: number;
  total: number;
  label: string;
  onPage: (page: number) => void;
  disabled?: boolean;
}) {
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const current = Math.min(page, pages - 1);
  return (
    <nav className="chapter-pagination" aria-label={label}>
      <small role="status">
        {total
          ? `${current * pageSize + 1}–${Math.min((current + 1) * pageSize, total)} of ${total}`
          : "0 results"}
      </small>
      {pages > 1 && (
        <div className="chapter-pagination-controls">
          <button
            className="quiet-button"
            aria-label={`Previous ${label}`}
            disabled={disabled || current === 0}
            onClick={() => onPage(current - 1)}
          >
            Previous
          </button>
          <label>
            <span className="chapter-sr-only">{label} page</span>
            <select
              value={current}
              disabled={disabled}
              onChange={(event) => onPage(Number(event.target.value))}
            >
              {Array.from({ length: pages }, (_, index) => (
                <option key={index} value={index}>
                  Page {index + 1} of {pages}
                </option>
              ))}
            </select>
          </label>
          <button
            className="quiet-button"
            aria-label={`Next ${label}`}
            disabled={disabled || current === pages - 1}
            onClick={() => onPage(current + 1)}
          >
            Next
          </button>
        </div>
      )}
    </nav>
  );
}
