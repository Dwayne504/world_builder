import { EntryDocumentPanel } from "./EntryDocumentPanel";
import { useEntryDescription } from "./useEntryDescription";
import type { FieldsController } from "./EntryFieldsPanel";
export function EntryDescriptionPanel(props: {
  projectId: string;
  entryId: string;
  disabled?: boolean;
  onController: (controller: FieldsController) => void;
  onRevision: (revision: number) => void;
  getRevision: () => number;
}) {
  const description = useEntryDescription(
    props.projectId,
    props.entryId,
    props.onRevision,
    props.getRevision,
  );
  return (
    <EntryDocumentPanel
      description={description}
      disabled={props.disabled}
      onController={props.onController}
    />
  );
}
