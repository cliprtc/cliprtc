import { Route, Routes } from 'react-router'
import Main from '@/views/main'
import Settings from '@/views/settings'

export default function Router() {
  return (
    <Routes>
      <Route path="/main" element={<Main />} />
      <Route path="/settings" element={<Settings />} />
    </Routes>
  )
}
