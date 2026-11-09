import { useSyncExternalStore, type AnchorHTMLAttributes, type MouseEvent } from 'react'

const EVENT_NAVIGASI = 'arsgis:navigasi'

function berlangganan(callback: () => void) {
  window.addEventListener('popstate', callback)
  window.addEventListener(EVENT_NAVIGASI, callback)
  return () => {
    window.removeEventListener('popstate', callback)
    window.removeEventListener(EVENT_NAVIGASI, callback)
  }
}

/** Path saat ini. Ikut berubah saat `navigate` atau tombol kembali/maju dipakai. */
export function usePathname(): string {
  return useSyncExternalStore(berlangganan, () => window.location.pathname, () => '/')
}

/** Pindah halaman di dalam aplikasi tanpa muat ulang. Hanya path internal (diawali "/"). */
export function navigate(href: string) {
  if (!href.startsWith('/') || href.startsWith('//')) {
    window.location.assign(href)
    return
  }
  if (window.location.pathname + window.location.search !== href) {
    window.history.pushState(null, '', href)
  }
  window.scrollTo(0, 0)
  window.dispatchEvent(new Event(EVENT_NAVIGASI))
}

type TautanProps = AnchorHTMLAttributes<HTMLAnchorElement> & { href: string }

/** Tautan yang berpindah halaman tanpa muat ulang. Klik dengan Ctrl, Shift, atau tengah tetap memakai perilaku browser. */
export function Tautan({ href, onClick, target, ...rest }: TautanProps) {
  function handleClick(e: MouseEvent<HTMLAnchorElement>) {
    onClick?.(e)
    if (
      e.defaultPrevented ||
      e.button !== 0 ||
      e.metaKey || e.ctrlKey || e.shiftKey || e.altKey ||
      (target && target !== '_self') ||
      !href.startsWith('/') || href.startsWith('//')
    ) {
      return
    }
    e.preventDefault()
    navigate(href)
  }
  return <a href={href} target={target} onClick={handleClick} {...rest} />
}
