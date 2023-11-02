"use client";

import { useState } from "react";
import {
  ClipboardIcon,
  ClipboardDocumentCheckIcon,
} from "@heroicons/react/24/outline";

interface CopyToClipboardButtonProps {
  text: string;
}

const CopyToClipboardButton = ({ text }: CopyToClipboardButtonProps) => {
  const [isCopied, setIsCopied] = useState(false);

  const copyToClipboard = () => {
    setIsCopied(true);
    navigator.clipboard.writeText(text);
    setTimeout(() => setIsCopied(false), 2000);
  };

  return (
    <button
      type="button"
      onClick={copyToClipboard}
      className="group p-1.5 text-xs bg-white/10 rounded-md transition-all"
    >
      {isCopied ? (
        <ClipboardDocumentCheckIcon className="rotate-12 w-4 h-4 text-slate-400 group-hover:text-slate-300 transition-all" />
      ) : (
        <ClipboardIcon className="w-4 h-4 text-slate-400 group-hover:text-slate-300 transition-all" />
      )}
    </button>
  );
};

export { CopyToClipboardButton };
