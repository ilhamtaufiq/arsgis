import { useQuery } from '@tanstack/react-query'
import { Map as MapIcon, LayoutDashboard, LogIn, ShieldCheck, Upload, Users, Wrench, type LucideIcon } from 'lucide-react'
import { fetchMe } from '../api'
import { itemAktif, menuUntukUser, type IdMenu } from '../shell/menu'
import { Tautan, usePathname } from '../shell/navigasi'
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  useSidebar,
} from '@/components/ui/sidebar'

const IKON: Record<IdMenu, LucideIcon> = {
  peta: MapIcon,
  dashboard: LayoutDashboard,
  pelanggan: Users,
  om: Wrench,
  impor: Upload,
  admin: ShieldCheck,
}

/** Sidebar navigasi arsgis, dipakai di semua halaman yang memakai shell. Item mengikuti peran pengguna. */
export function AppSidebar() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const path = usePathname()
  const aktif = itemAktif(path)
  const menu = menuUntukUser(me.data ?? null)
  const { setOpenMobile, isMobile } = useSidebar()

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader>
        <div className="px-2 py-1.5">
          <p className="text-base font-semibold tracking-tight">arsgis</p>
          <p className="text-xs text-muted-foreground">WebGIS perpipaan SPAM</p>
        </div>
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Menu</SidebarGroupLabel>
          <SidebarGroupContent>
            {me.isPending ? null : menu.length === 0 ? (
              <Tautan href="/masuk" className="flex items-center gap-2 px-2 py-1.5 text-sm text-muted-foreground hover:underline">
                <LogIn className="size-4" aria-hidden="true" />
                <span>Masuk untuk membuka menu</span>
              </Tautan>
            ) : (
              <SidebarMenu>
                {menu.map((item) => {
                  const Ikon = IKON[item.id]
                  return (
                    <SidebarMenuItem key={item.id}>
                      <SidebarMenuButton
                        asChild
                        isActive={aktif === item.id}
                        tooltip={item.judul}
                        onClick={() => { if (isMobile) setOpenMobile(false) }}
                      >
                        <Tautan href={item.href} aria-current={aktif === item.id ? 'page' : undefined}>
                          <Ikon aria-hidden="true" />
                          <span>{item.judul}</span>
                        </Tautan>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  )
                })}
              </SidebarMenu>
            )}
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarRail />
    </Sidebar>
  )
}
