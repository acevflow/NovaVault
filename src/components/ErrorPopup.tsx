import { AlertCircle, X } from "lucide-react";
import { useEffect, useState } from "react";

interface ErrorPopupProps {
  open: boolean;
  title?: string;
  message: string;
  onClose: () => void;
}

function ErrorPopup({
  open,
  title = "Something went wrong",
  message,
  onClose,
}: ErrorPopupProps) {
  const [visible, setVisible] = useState(open);
  const [closing, setClosing] = useState(false);

  useEffect(() => {
    if (open) {
      setVisible(true);
      setClosing(false);
      return;
    }

    if (!visible) {
      return;
    }

    setClosing(true);

    const timeout = window.setTimeout(() => {
      setVisible(false);
      setClosing(false);
    }, 180);

    return () => {
      window.clearTimeout(timeout);
    };
  }, [open, visible]);

  if (!visible) {
    return null;
  }

  const handleClose = () => {
    if (closing) {
      return;
    }

    setClosing(true);

    window.setTimeout(() => {
      onClose();
    }, 180);
  };

  return (
    <div
      className={`fixed inset-0 z-50 flex items-center justify-center overflow-y-auto p-4 min-[600px]:p-5 ${
        closing ? "nv-error-overlay-out" : "nv-error-overlay-in"
      }`}
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          handleClose();
        }
      }}
    >
      <div
        className={`w-full max-w-105 max-h-[calc(100vh-32px)] overflow-y-auto rounded-[20px] border border-(--color-border-strong) bg-(--color-surface) p-4 shadow-xl min-[600px]:rounded-[22px] min-[600px]:p-5 ${
          closing ? "nv-error-popup-out" : "nv-error-popup-in"
        }`}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="error-popup-title"
        aria-describedby="error-popup-message"
        onMouseDown={(event) => {
          event.stopPropagation();
        }}
      >
        <div className="flex items-start gap-2.5 min-[600px]:gap-3">
          <AlertCircle className="mt-0.5 h-5 w-5 shrink-0 text-(--color-danger) min-[600px]:h-6 min-[600px]:w-6" />
          <div className="min-w-0 flex-1">
            <h2
              id="error-popup-title"
              className="text-[17px] font-semibold text-(--color-text-primary) min-[600px]:text-[18px]"
            >
              {title}
            </h2>
            <p
              id="error-popup-message"
              className="mt-1.5 wrap-break-word text-[14px] leading-normal text-(--color-text-secondary) min-[600px]:text-[15px]"
            >
              {message}
            </p>
          </div>
          <button
            type="button"
            aria-label="Close error"
            className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-(--color-text-muted) transition hover:bg-(--color-surface-hover) hover:text-(--color-text-primary) active:bg-(--color-surface-active)"
            onClick={handleClose}
          >
            <X className="h-5 w-5" />
          </button>
        </div>
        <div className="mt-4 flex justify-end min-[600px]:mt-5">
          <button
            type="button"
            className="rounded-2xl bg-(--color-accent) px-4 py-2 text-[15px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active) min-[600px]:text-[16px]"
            onClick={handleClose}
          >
            OK
          </button>
        </div>
      </div>
    </div>
  );
}

export default ErrorPopup;
