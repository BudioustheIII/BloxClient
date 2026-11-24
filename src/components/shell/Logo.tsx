import logoUrl from "../../../assets/brand/logo.svg";

/** The source artwork has wide padding, so it is cropped to stay legible at title bar size. */
export function Logo({ size = 20 }: { size?: number }) {
  return (
    <span
      aria-hidden="true"
      className="inline-block shrink-0 overflow-hidden rounded-[5px] bg-black"
      style={{ width: size, height: size }}
    >
      <img src={logoUrl} width={size} height={size} alt="" draggable={false} className="scale-[1.6]" />
    </span>
  );
}
