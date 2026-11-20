# One day of the classic Outlook calendar running in this session, or the day's bounds alone where none runs
# (docs/design.md, "A person's Outlook day"). Run on a person's own node through the node service's runs, in the
# person's interactive session. The day comes in BMO_CALENDAR_DAY, YYYY-MM-DD on this PC's wall clock, today where
# it is empty. COM is asked for Outlook only where its process runs, since asking for one that does not run starts
# it. The answer is one JSON object on the last line, its UTF-8 bytes written in base64 so that no console code
# page changes them.
$ErrorActionPreference = 'Stop'
$invariant = [Globalization.CultureInfo]::InvariantCulture
$localShape = "yyyy-MM-dd'T'HH:mm:ss"
$utcShape = "yyyy-MM-dd'T'HH:mm:ss'Z'"
$named = $env:BMO_CALENDAR_DAY
if ([string]::IsNullOrEmpty($named)) {
  $day = [DateTime]::Today
} else {
  $day = [DateTime]::ParseExact($named, 'yyyy-MM-dd', $invariant)
}
$next = $day.AddDays(1)
$answer = [ordered]@{
  date     = $day.ToString('yyyy-MM-dd', $invariant)
  zone     = [TimeZoneInfo]::Local.Id
  startUtc = [DateTime]::SpecifyKind($day, 'Local').ToUniversalTime().ToString($utcShape, $invariant)
  endUtc   = [DateTime]::SpecifyKind($next, 'Local').ToUniversalTime().ToString($utcShape, $invariant)
  outlook  = 'not-running'
  why      = ''
  events   = @()
}
if (Get-Process -Name 'OUTLOOK' -ErrorAction SilentlyContinue) {
  try {
    $outlook = [Runtime.InteropServices.Marshal]::GetActiveObject('Outlook.Application')
    # The default calendar's items, sorted before their repeats are unfolded, as Outlook requires; the filter
    # takes the culture's own short date and time, which is how Outlook reads a date in it, and a day more on
    # each side, the exact span judged item by item below.
    $items = $outlook.Session.GetDefaultFolder(9).Items
    $items.Sort('[Start]')
    $items.IncludeRecurrences = $true
    $culture = [Globalization.CultureInfo]::CurrentCulture
    $from = $day.AddDays(-1).ToString('g', $culture)
    $to = $next.AddDays(1).ToString('g', $culture)
    $found = $items.Restrict("[Start] < '$to' AND [End] > '$from'")
    $events = New-Object System.Collections.ArrayList
    $item = $found.GetFirst()
    while ($null -ne $item -and $item.Start -lt $next -and $events.Count -lt 500) {
      $meets = $item.End -gt $day -or $item.Start -ge $day
      # 5 and 7: a meeting its organizer cancelled, as the organizer's and an attendee's copy mark it.
      $cancelled = $item.MeetingStatus -eq 5 -or $item.MeetingStatus -eq 7
      if ($meets -and -not $cancelled) {
        $organizer = [string]$item.Organizer
        $attendees = 0
        foreach ($recipient in $item.Recipients) {
          # 1 and 2: a required and an optional attendee; 3, a resource, is a room.
          if (($recipient.Type -eq 1 -or $recipient.Type -eq 2) -and $recipient.Name -ne $organizer) {
            $attendees++
          }
        }
        [void]$events.Add([ordered]@{
          title     = [string]$item.Subject
          start     = $item.Start.ToString($localShape, $invariant)
          end       = $item.End.ToString($localShape, $invariant)
          allDay    = [bool]$item.AllDayEvent
          organizer = $organizer
          attendees = $attendees
          room      = [string]$item.Location
        })
      }
      $item = $found.GetNext()
    }
    $answer.events = $events.ToArray()
    $answer.outlook = 'running'
  } catch {
    $answer.outlook = 'unreachable'
    $answer.why = $_.Exception.Message
    $answer.events = @()
  }
}
$json = ConvertTo-Json -InputObject $answer -Depth 4 -Compress
[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($json))
