import { useEffect, useLayoutEffect, useRef } from "react";
import { EditorContent, useEditor, useEditorState, type JSONContent } from "@tiptap/react";
import { writingExtensions } from "./writingSchema";
export interface WritingPosition {
  from: number;
  to: number;
}
export function RichTextEditor({
  locked = false,
  content,
  label,
  onChange,
  onBlur,
  position,
  onPosition,
}: {
  locked?: boolean;
  content: JSONContent;
  label: string;
  onChange: (content: JSONContent) => void;
  onBlur: () => void;
  position?: WritingPosition;
  onPosition: (position: WritingPosition) => void;
}) {
  const callbacks = useRef({ onChange, onBlur, onPosition });
  callbacks.current = { onChange, onBlur, onPosition };
  // Content is supplied only on mount. Save acknowledgements never reset selection or undo.
  const editor = useEditor({
    extensions: writingExtensions(),
    content,
    editable: !locked,
    editorProps: {
      attributes: {
        role: "textbox",
        "aria-label": label,
        "aria-multiline": "true",
        spellcheck: "true",
      },
    },
    onUpdate: ({ editor }) => callbacks.current.onChange(editor.getJSON()),
    onBlur: () => callbacks.current.onBlur(),
    onSelectionUpdate: ({ editor }) =>
      callbacks.current.onPosition({
        from: editor.state.selection.from,
        to: editor.state.selection.to,
      }),
  });
  useLayoutEffect(() => {
    editor?.setEditable(!locked, false);
  }, [editor, locked]);
  const selection = useEditorState({
    editor,
    selector: ({ editor }) => ({
      bold: editor?.isActive("bold"),
      italic: editor?.isActive("italic"),
      heading: editor?.isActive("heading", { level: 2 }),
      bullet: editor?.isActive("bulletList"),
      ordered: editor?.isActive("orderedList"),
      quote: editor?.isActive("blockquote"),
    }),
  });
  const restored = useRef(false);
  useEffect(() => {
    if (!editor || restored.current) return;
    restored.current = true;
    if (position)
      editor.commands.setTextSelection({
        from: Math.min(position.from, editor.state.doc.content.size),
        to: Math.min(position.to, editor.state.doc.content.size),
      });
  }, [editor, position]);
  if (!editor) return <p role="status">Opening writing surface…</p>;
  return (
    <div className="rich-writing">
      <fieldset
        disabled={locked}
        className="writing-tools"
        role="toolbar"
        aria-label={`${label} formatting`}
        onMouseDown={(e) => e.preventDefault()}
      >
        <button
          title="Bold (Ctrl+B)"
          aria-label="Bold"
          aria-pressed={!!selection?.bold}
          onClick={() => editor.chain().focus().toggleBold().run()}
        >
          <strong>B</strong>
        </button>
        <button
          title="Italic (Ctrl+I)"
          aria-label="Italic"
          aria-pressed={!!selection?.italic}
          onClick={() => editor.chain().focus().toggleItalic().run()}
        >
          <em>I</em>
        </button>
        <button
          aria-label="Heading"
          aria-pressed={!!selection?.heading}
          onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        >
          Heading
        </button>
        <button
          aria-label="Bullet list"
          aria-pressed={!!selection?.bullet}
          onClick={() => editor.chain().focus().toggleBulletList().run()}
        >
          • List
        </button>
        <button
          aria-label="Numbered list"
          aria-pressed={!!selection?.ordered}
          onClick={() => editor.chain().focus().toggleOrderedList().run()}
        >
          1. List
        </button>
        <button
          aria-label="Quote"
          aria-pressed={!!selection?.quote}
          onClick={() => editor.chain().focus().toggleBlockquote().run()}
        >
          Quote
        </button>
        <button aria-label="Undo writing" onClick={() => editor.chain().focus().undo().run()}>
          Undo
        </button>
        <button aria-label="Redo writing" onClick={() => editor.chain().focus().redo().run()}>
          Redo
        </button>
      </fieldset>
      <EditorContent editor={editor} />
    </div>
  );
}
