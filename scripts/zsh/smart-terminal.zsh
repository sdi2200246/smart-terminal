# Ensure the Zsh system module is loaded for low-level I/O operations
zmodload zsh/system

# ---------------------------------------------------------------------
# Global State Variables
# ---------------------------------------------------------------------
AI_LAST_SUGGESTION=""
AI_LAST_REVERSIBILITY=""
AI_LAST_DESCRIPTION=""
AI_BUFFER_OWNER=""
AI_EMPTY_BUFFER_ONLY=0
AI_LOADING=""
AI_DOTS=0
AI_FETCH_FD=""
AI_FETCH_BUFFER=""
AI_FETCH_LINES=()
AI_TICK_FD=""

if [[ -z "$AI_HANDOFF_DIR" || ! -d "$AI_HANDOFF_DIR" || "$SMART_TERMINAL_SUGGESTION_DIR" != "$AI_HANDOFF_DIR" ]]; then
  AI_HANDOFF_DIR="$(mktemp -d "${TMPDIR:-/tmp}/smart-terminal-${UID}-XXXXXXXXXX")" || {
    print -u2 "smart-terminal: unable to create a private suggestion handoff directory"
    return 1
  }
  export SMART_TERMINAL_SUGGESTION_DIR="$AI_HANDOFF_DIR"
fi

# ---------------------------------------------------------------------
# UI and Color Formatting Helpers
# ---------------------------------------------------------------------
ai_reversibility_color() {
  case "$1" in
    Full)         echo "fg=46"  ;;
    Mostly)       echo "fg=39"  ;;
    Partial)      echo "fg=226" ;;
    Hard)         echo "fg=202" ;;
    Irreversible) echo "fg=196" ;;
    *)            echo "fg=8"   ;;
  esac
}

# Ghost text injection engine (called by the Zsh line redraw hook)
ai_ghost() {
  region_highlight=("${(@)region_highlight:#*ghost_highlight*}")

  if [[ -n "$AI_LOADING" ]]; then
    POSTDISPLAY="  # $AI_LOADING"
    local start=$#BUFFER
    local end=$(( start + ${#POSTDISPLAY} ))
    region_highlight+=("$start $end fg=242,italic # ghost_highlight")
    return
  fi

  if [[ -n "$AI_LAST_SUGGESTION" ]]; then
    if (( AI_EMPTY_BUFFER_ONLY )) && [[ -n "$BUFFER" ]]; then
      AI_LAST_SUGGESTION=""
      AI_LAST_REVERSIBILITY=""
      AI_LAST_DESCRIPTION=""
      AI_BUFFER_OWNER=""
      AI_EMPTY_BUFFER_ONLY=0
      POSTDISPLAY=""
      return
    fi
    if [[ "$BUFFER" != "$AI_BUFFER_OWNER"* ]]; then
      AI_LAST_SUGGESTION=""
      AI_LAST_REVERSIBILITY=""
      AI_LAST_DESCRIPTION=""
      AI_EMPTY_BUFFER_ONLY=0
      POSTDISPLAY=""
      return
    fi
  fi

  if [[ -n "$AI_LAST_SUGGESTION" ]]; then
    local display_text="" desc_text="" desc_separator="  # "
    if [[ "$AI_LAST_SUGGESTION" == "$BUFFER"* ]]; then
      display_text="${AI_LAST_SUGGESTION#$BUFFER}"
    else
      display_text=" -> $AI_LAST_SUGGESTION"
    fi
    [[ "$AI_LAST_SUGGESTION" == *$'\n'* ]] && desc_separator=$'\n  # '
    [[ -n "$AI_LAST_DESCRIPTION" ]] && desc_text="${desc_separator}${AI_LAST_DESCRIPTION}"
    if [[ -n "$display_text" ]]; then
      POSTDISPLAY="${display_text}${desc_text}"
      local start=$#BUFFER
      local cmd_end=$(( start + ${#display_text} ))
      local desc_end=$(( cmd_end + ${#desc_text} ))
      local desc_color="$(ai_reversibility_color "$AI_LAST_REVERSIBILITY")"
      region_highlight+=("$start $cmd_end fg=242 # ghost_highlight")
      region_highlight+=("$cmd_end $desc_end $desc_color # ghost_highlight")
    else
      POSTDISPLAY=""
    fi
  else
    POSTDISPLAY=""
  fi
}

# ---------------------------------------------------------------------
# Zsh Line Editor (ZLE) Asynchronous Display Bridge
# ---------------------------------------------------------------------
# This official widget forces Zsh to instantly re-render changes made 
# in background file descriptor handlers.
_ai_redisplay_ghost() {
  ai_ghost
  zle -R
}
zle -N _ai_redisplay_ghost

# ---------------------------------------------------------------------
# Loading Ticker Animation ("thinking . . .")
# ---------------------------------------------------------------------
ai_stop_ticker() {
  if [[ -n "$AI_TICK_FD" ]]; then
    zle -F $AI_TICK_FD 2>/dev/null
    exec {AI_TICK_FD}<&- 
    AI_TICK_FD=""
  fi
}

ai_tick_handler() {
  local fd=$1 chunk i dots
  if ! sysread -i $fd chunk 2>/dev/null; then
    ai_stop_ticker
    return
  fi
  
  dots=""
  for ((i=0; i< AI_DOTS % 4; i++)); do dots+=" ."; done
  AI_LOADING="thinking${dots}"
  AI_DOTS=$(( AI_DOTS + 1 ))  

  # Trigger instant display refresh for the ticker
  zle _ai_redisplay_ghost
}

ai_start_ticker() {
  ai_stop_ticker
  AI_DOTS=1
  AI_LOADING="thinking"
  exec {AI_TICK_FD}< <(while sleep 0.3; do print tick; done)
  zle -F $AI_TICK_FD ai_tick_handler
}

# ---------------------------------------------------------------------
# Rust Binary Stream Handlers & Pipeline Cleanup
# ---------------------------------------------------------------------
_ai_cleanup_fetch() {
  local fd=$1
  local protocol
  zle -F $fd 2>/dev/null
  exec {fd}<&- 
  AI_FETCH_FD=""
  
  [[ -n "$AI_FETCH_BUFFER" ]] && AI_FETCH_LINES+=("$AI_FETCH_BUFFER")
  AI_FETCH_BUFFER=""
  
  if (( ${#AI_FETCH_LINES} >= 3 )); then
    protocol="${AI_FETCH_LINES[1]}"
    if [[ "$protocol" != "SMART_TERMINAL_NEXT_CMD_V1" ]]; then
      AI_LAST_SUGGESTION=""
      AI_LAST_DESCRIPTION=""
      AI_LAST_REVERSIBILITY=""
      POSTDISPLAY=""
      zle -M "smart-terminal: outdated next-cmd output; rebuild/reinstall the current binary"
    else
      AI_LAST_DESCRIPTION="${AI_FETCH_LINES[2]}"
      AI_LAST_REVERSIBILITY="${AI_FETCH_LINES[3]}"
      AI_LAST_SUGGESTION="${(F)AI_FETCH_LINES[4,-1]}"
    fi
  fi
  AI_FETCH_LINES=()
  ai_stop_ticker
  AI_LOADING=""
  
  zle _ai_redisplay_ghost
}

ai_fetch_handler() {
  local fd=$1 event=$2 chunk
  
  if ! sysread -i $fd chunk 2>/dev/null; then
    _ai_cleanup_fetch $fd
    return
  fi

  AI_FETCH_BUFFER+="$chunk"
  while [[ "$AI_FETCH_BUFFER" == *$'\n'* ]]; do
    AI_FETCH_LINES+=("${AI_FETCH_BUFFER%%$'\n'*}")
    AI_FETCH_BUFFER="${AI_FETCH_BUFFER#*$'\n'}"
  done

}

# ---------------------------------------------------------------------
# Primary Core Core Interactive Actions
# ---------------------------------------------------------------------
ai_fetch_suggestion() {
  [[ -n "$AI_FETCH_FD" ]] && return
  AI_BUFFER_OWNER="$BUFFER"
  AI_LAST_SUGGESTION=""
  AI_LAST_DESCRIPTION=""
  AI_LAST_REVERSIBILITY=""
  AI_EMPTY_BUFFER_ONLY=0
  AI_FETCH_BUFFER=""
  AI_FETCH_LINES=()
  export AI_CONTEXT_HISTORY="$(history -n -20)"

  # Starts Rust binary. '< /dev/null' explicitly cuts standard input to avoid deadlocks.
  exec {AI_FETCH_FD}< <(smart-terminal next-cmd "$BUFFER" < /dev/null 2>/dev/null)
  zle -F $AI_FETCH_FD ai_fetch_handler

  ai_start_ticker
  zle reset-prompt
}

ai_accept_suggestion() {
  if [[ -n "$AI_LAST_SUGGESTION" ]]; then
    if (( AI_EMPTY_BUFFER_ONLY )) && [[ -n "$BUFFER" ]]; then
      ai_clear_suggestion
      return
    fi
    BUFFER="$AI_LAST_SUGGESTION"
    CURSOR=${#BUFFER}
    AI_LAST_SUGGESTION=""
    AI_LAST_REVERSIBILITY=""
    AI_LAST_DESCRIPTION=""
    AI_BUFFER_OWNER=""
    AI_EMPTY_BUFFER_ONLY=0
    POSTDISPLAY=""
    region_highlight=()
    zle redisplay
  fi
}

ai_clear_suggestion() {
  if [[ -n "$AI_FETCH_FD" ]]; then
    zle -F $AI_FETCH_FD 2>/dev/null
    exec {AI_FETCH_FD}<&- 2>/dev/null
    AI_FETCH_FD=""
  fi
  ai_stop_ticker
  AI_FETCH_BUFFER=""
  AI_FETCH_LINES=()
  AI_LAST_SUGGESTION=""
  AI_LAST_REVERSIBILITY=""
  AI_LAST_DESCRIPTION=""
  AI_BUFFER_OWNER=""
  AI_EMPTY_BUFFER_ONLY=0
  AI_LOADING=""
  POSTDISPLAY=""
  region_highlight=()
  zle redisplay
}

_ai_receive_investigator_suggestion() {
  [[ -n "$AI_HANDOFF_DIR" ]] || return

  local handoff_file="$AI_HANDOFF_DIR/investigator-command"
  [[ -f "$handoff_file" ]] || return

  local -a suggestion_data
  suggestion_data=("${(@f)$(<"$handoff_file")}")
  rm -f "$handoff_file"

  (( ${#suggestion_data} >= 4 )) || return
  [[ "${suggestion_data[1]}" == "SMART_TERMINAL_NEXT_CMD_V1" ]] || return
  [[ -n "${suggestion_data[2]}" ]] || return
  case "${suggestion_data[3]}" in
    Full|Mostly|Partial|Hard|Irreversible) ;;
    *) return ;;
  esac

  AI_LAST_SUGGESTION="${(F)suggestion_data[4,-1]}"
  [[ -n "$AI_LAST_SUGGESTION" ]] || return
  AI_LAST_DESCRIPTION="${suggestion_data[2]}"
  AI_LAST_REVERSIBILITY="${suggestion_data[3]}"
  AI_BUFFER_OWNER=""
  AI_EMPTY_BUFFER_ONLY=1
}

# ---------------------------------------------------------------------
# Widget Registrations and Keybindings
# ---------------------------------------------------------------------
zle -N ai_fetch_suggestion
zle -N ai_accept_suggestion
zle -N ai_clear_suggestion
zle -N ai_ghost
zle -N ai_tick_handler
zle -N ai_fetch_handler

# Hook into the Zsh line-drawing interface to continuously keep ghost highlights accurate
autoload -Uz add-zle-hook-widget
add-zle-hook-widget line-pre-redraw ai_ghost
add-zle-hook-widget zle-line-init _ai_redisplay_ghost

# Define hotkeys
bindkey '^G' ai_fetch_suggestion  # Ctrl + G to request suggestions
bindkey '^F' ai_accept_suggestion # Ctrl + F to accept suggestions
bindkey '^B' ai_clear_suggestion  # Ctrl + B to dismiss suggestions

export ERR_STREAM="/tmp/err_stream_$$.log"
export ERR_LAST="/tmp/last_err_$$.log"
touch "$ERR_STREAM" "$ERR_LAST"

exec 2> >(tee -a "$ERR_STREAM" >&2)

_snapshot_err() {
  sleep 0.05
  cp "$ERR_STREAM" "$ERR_LAST" 2>/dev/null
  : > "$ERR_STREAM"
}

autoload -Uz add-zsh-hook

zshexit() {
  rm -f "$ERR_STREAM" "$ERR_LAST" "$AI_HANDOFF_DIR/investigator-command"
  rmdir "$AI_HANDOFF_DIR" 2>/dev/null
}

add-zsh-hook precmd _snapshot_err
add-zsh-hook precmd _ai_receive_investigator_suggestion