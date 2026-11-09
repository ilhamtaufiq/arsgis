import type { ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { AuthPanel } from '../Auth'
import { fetchMe } from '../api'
import { AppSidebar } from '@/components/app-sidebar'
import { Separator } from '@/components/ui/separator'
import { SidebarProvider, SidebarTrigger } from '@/components/ui/sidebar'
import { cn } from '@/lib/utils'

type Props = {
  /** Nama halaman aktif, tampil di header. */
  title: string
  /** Peta dan halaman berlayar penuh: konten mengisi sisa tinggi viewport, tidak ikut menggulir. */
  penuhLayar?: boolean
  /** Kontrol di kanan header. Standarnya tombol masuk atau keluar. */
  aksi?: ReactNode
  children: ReactNode
}

/**
 * Kerangka halaman bersama: sidebar navigasi dan header. Halaman login dan landing tidak memakai ini.
 * Di ponsel sidebar menjadi panel yang dibuka dari tombol di header.
 */
export function AppShell({ title, penuhLayar = false, aksi, children }: Props) {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })

  return (
    <SidebarProvider className={cn(penuhLayar && 'h-svh overflow-hidden')}>
      <AppSidebar />
      <div className={cn('flex min-w-0 flex-1 flex-col bg-background', penuhLayar ? 'h-svh overflow-hidden' : 'min-h-svh')}>
        <header className="flex h-14 shrink-0 items-center gap-2 border-b bg-background px-4">
          <SidebarTrigger className="-ml-1" aria-label="Buka atau tutup menu navigasi" />
          <Separator orientation="vertical" className="mr-2 h-4" />
          <p className="truncate text-sm font-medium">{title}</p>
          <div className="ml-auto flex items-center gap-2">
            {aksi ?? <AuthPanel user={me.data ?? null} inline />}
          </div>
        </header>
        <div className={penuhLayar ? 'relative min-h-0 flex-1' : 'flex min-w-0 flex-1 flex-col'}>{children}</div>
      </div>
    </SidebarProvider>
  )
}
