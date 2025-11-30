import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router'
import Router from './router'
import { init } from './utils/init'
import './App.css'

const app = document.getElementById('app')!

init().then(() => {
  ReactDOM.createRoot(app).render(
    <BrowserRouter>
      <React.StrictMode>
        <Router />
      </React.StrictMode>
    </BrowserRouter>,
  )
})
