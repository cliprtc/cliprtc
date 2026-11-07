import antfu from '@antfu/eslint-config'

export default antfu({
  react: true,
  ignores: ['src-tauri/**', 'dist/**', 'node_modules/**'],
  rules: {
    'no-console': 'warn',
    'curly': ['error', 'multi-line', 'consistent'],
  },
})
