import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
  type UIEventHandler,
} from "react";

export type ScrollCue = {
  top: boolean;
  bottom: boolean;
};

type ScrollRegionProps = {
  children: ReactNode;
  className?: string;
  contentClassName?: string;
  "aria-label"?: string;
};

function joinClassNames(...parts: Array<string | undefined | false>): string {
  return parts.filter(Boolean).join(" ");
}

/**
 * PDU-style scroll area: native scrollbar is hidden.
 * Soft gradient + chevron cues show when more content is available above/below.
 */
export function ScrollRegion({
  children,
  className,
  contentClassName,
  "aria-label": ariaLabel,
}: ScrollRegionProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [scrollCue, setScrollCue] = useState<ScrollCue>({ top: false, bottom: false });

  const updateScrollCue = useCallback(() => {
    const element = scrollRef.current;
    if (!element) {
      setScrollCue({ top: false, bottom: false });
      return;
    }

    const overflow = element.scrollHeight > element.clientHeight + 1;
    const atTop = element.scrollTop <= 1;
    const atBottom = element.scrollTop + element.clientHeight >= element.scrollHeight - 1;
    const nextCue = {
      top: overflow && !atTop,
      bottom: overflow && !atBottom,
    };

    setScrollCue((current) =>
      current.top === nextCue.top && current.bottom === nextCue.bottom ? current : nextCue,
    );
  }, []);

  // Observe the viewport once — do not rebind when `children` identity changes
  // (that was resetting layout cues and contributing to sidebar jump on preview refresh).
  useEffect(() => {
    updateScrollCue();
    const element = scrollRef.current;
    if (!element || typeof ResizeObserver === "undefined") {
      return;
    }
    const observer = new ResizeObserver(() => updateScrollCue());
    observer.observe(element);
    if (element.firstElementChild) {
      observer.observe(element.firstElementChild);
    }
    return () => observer.disconnect();
  }, [updateScrollCue]);

  useEffect(() => {
    updateScrollCue();
  }, [children, updateScrollCue]);

  const onScroll: UIEventHandler<HTMLDivElement> = () => {
    updateScrollCue();
  };

  return (
    <div className={joinClassNames("scroll-region", className)}>
      {scrollCue.top ? (
        <div className="scroll-cue scroll-cue-top" aria-hidden="true">
          <span className="scroll-cue-chevron scroll-cue-chevron-up" />
        </div>
      ) : null}
      <div
        ref={scrollRef}
        aria-label={ariaLabel}
        onScroll={onScroll}
        className="scroll-region-viewport"
      >
        <div className={joinClassNames("scroll-region-content", contentClassName)}>{children}</div>
      </div>
      {scrollCue.bottom ? (
        <div className="scroll-cue scroll-cue-bottom" aria-hidden="true">
          <span className="scroll-cue-chevron scroll-cue-chevron-down" />
        </div>
      ) : null}
    </div>
  );
}
