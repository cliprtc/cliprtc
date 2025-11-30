import { Route, Routes } from 'react-router'
import Settings from '@/views/settings'

export default function Router() {
  return (
    <Routes>
      <Route path="/settings" element={<Settings />} />
    </Routes>
  )
}
