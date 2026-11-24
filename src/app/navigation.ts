import {
  Activity,
  Compass,
  Heart,
  House,
  Library,
  Settings,
  UserRound,
  Users,
  type LucideIcon,
} from "lucide-react";
import type { PageId } from "@/lib/bindings/PageId";

export interface NavItem {
  id: PageId;
  label: string;
  icon: LucideIcon;
}

export const NAV_ITEMS: readonly NavItem[] = [
  { id: "home", label: "Home", icon: House },
  { id: "discover", label: "Discover", icon: Compass },
  { id: "library", label: "Library", icon: Library },
  { id: "favorites", label: "Favorites", icon: Heart },
  { id: "friends", label: "Friends", icon: Users },
  { id: "performance", label: "Performance", icon: Activity },
  { id: "accounts", label: "Accounts", icon: UserRound },
  { id: "settings", label: "Settings", icon: Settings },
];
