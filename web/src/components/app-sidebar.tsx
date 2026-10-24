import { Map as MapIcon, LayoutDashboard, ShieldCheck } from 'lucide-react'
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
} from '@/components/ui/sidebar'

const MENU = [
  { title: 'Peta', href: '/', icon: MapIcon },
  { title: 'Dashboard', href: '/dashboard', icon: LayoutDashboard },
  { title: 'Admin', href: '/admin', icon: ShieldCheck },
]

/** Sidebar navigasi arsgis. Item aktif mengikuti path halaman. */
export function AppSidebar() {
  const path = window.location.pathname.replace(/\/+$/, '') || '/'

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
            <SidebarMenu>
              {MENU.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton asChild isActive={path === item.href} tooltip={item.title}>
                    <a href={item.href}>
                      <item.icon />
                      <span>{item.title}</span>
                    </a>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarRail />
    </Sidebar>
  )
}
