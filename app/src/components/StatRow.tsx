type Props = {
  label: string
  value: string
  hint?: string
}

export function StatRow({ label, value, hint }: Props) {
  return (
    <div className="stat-row">
      <dt>{label}</dt>
      <dd>
        <span className="stat-value">{value}</span>
        {hint ? <span className="stat-hint">{hint}</span> : null}
      </dd>
    </div>
  )
}
