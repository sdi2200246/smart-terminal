# Ensure the Zsh system module is loaded for low-level I/O operations
zmodload zsh/system

# ---------------------------------------------------------------------
# Global State Variables
# ---------------------------------------------------------------------
AI_LAST_SUGGESTION=""
AI_LAST_REVERSIBILITY=""
AI_LAST_DESCRIPTION=""
AI_BUFFER_OWNER=""
AI_LOADING=""
AI_DOTS=0
AI_FETCH_FD=""
AI_FETCH_BUFFER=""
AI_FETCH_LINES=()
AI_TICK_FD=""

# ---------------------------------------------------------------------
# UI and Color Formatting Helpers
# ---------------------------------------------------------------------
ai_reversibility_color() {
  case "$1" in
    Full)         echo "fg=22"  ;;
    Mostly)       echo "fg=23"  ;;
    Partial)      echo "fg=58"  ;;
    Hard)         echo "fg=88"  ;;
    Irreversible) echo "fg=124" ;;
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
    if [[ "$BUFFER" != "$AI_BUFFER_OWNER"* ]]; then
      AI_LAST_SUGGESTION=""
      AI_LAST_REVERSIBILITY=""
      AI_LAST_DESCRIPTION=""
      POSTDISPLAY=""
      return
    fi
  fi

  if [[ -n "$AI_LAST_SUGGESTION" ]]; then
    local display_text="" desc_text=""
    if [[ "$AI_LAST_SUGGESTION" == "$BUFFER"* ]]; then
      display_text="${AI_LAST_SUGGESTION#$BUFFER}"
    else
      display_text=" -> $AI_LAST_SUGGESTION"
    fi
    [[ -n "$AI_LAST_DESCRIPTION" ]] && desc_text="  # $AI_LAST_DESCRIPTION"
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
    exec {AI_TICK_FD}<&- 2>/dev/null
    AI_TICK_FD=""
  fi
}

ai_tick_handler() {
  local fd=$1 chunk i dots
  if ! sysread -i $fd chunk 2>/dev/null; then
    ai_stop_ticker
    return
  fi
  
  AI_DOTS=$( (AI_DOTS + 1 ))
  dots=""
  for ((i=0; i< $( (AI_DOTS % 3) ); i++)); do dots+=" ."; done
  AI_LOADING="thinking${dots}"
  
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
  zle -F $fd 2>/dev/null
  exec {fd}<&- 2>/dev/null
  AI_FETCH_FD=""
  
  [[ -n "$AI_FETCH_BUFFER" ]] && AI_FETCH_LINES+=("$AI_FETCH_BUFFER")
  AI_FETCH_BUFFER=""
  
  if (( ${#AI_FETCH_LINES} >= 3 )); then
    AI_LAST_SUGGESTION="${AI_FETCH_LINES[1]}"
    AI_LAST_DESCRIPTION="${AI_FETCH_LINES[2]}"
    AI_LAST_REVERSIBILITY="${AI_FETCH_LINES[3]}"
  fi
  AI_FETCH_LINES=()
  ai_stop_ticker
  AI_LOADING=""
  
  zle _ai_redisplay_ghost
}

ai_fetch_handler() {
  local fd=$1 event=$2 chunk
  
  if [[ "$event" == "hup" || "$event" == "err" ]]; then
    _ai_cleanup_fetch $fd
    return
  fi

  if ! sysread -i $fd chunk 2>/dev/null; then
    _ai_cleanup_fetch $fd
    return
  fi

  AI_FETCH_BUFFER+="$chunk"
  while [[ "$AI_FETCH_BUFFER" == *$'\n'* ]]; do
    AI_FETCH_LINES+=("${AI_FETCH_BUFFER%%$'\n'*}")
    AI_FETCH_BUFFER="${AI_FETCH_BUFFER#*$'\n'}"
  done

  # When all 3 flushed rows from Rust cross the pipe, process them immediately
  if (( ${#AI_FETCH_LINES} >= 3 )); then
    zle -F $fd 2>/dev/null
    exec {fd}<&- 2>/dev/null
    AI_FETCH_FD=""
    
    AI_LAST_SUGGESTION="${AI_FETCH_LINES[1]}"
    AI_LAST_DESCRIPTION="${AI_FETCH_LINES[2]}"
    AI_LAST_REVERSIBILITY="${AI_FETCH_LINES[3]}"
    
    AI_FETCH_BUFFER=""
    AI_FETCH_LINES=()
    ai_stop_ticker
    AI_LOADING=""
    
    # Run the official display bridge widget to draw the suggestion instantly
    zle _ai_redisplay_ghost
  fi
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
    BUFFER="$AI_LAST_SUGGESTION"
    CURSOR=${#BUFFER}
    AI_LAST_SUGGESTION=""
    AI_LAST_REVERSIBILITY=""
    AI_LAST_DESCRIPTION=""
    AI_BUFFER_OWNER=""
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
  AI_LOADING=""
  POSTDISPLAY=""
  region_highlight=()
  zle redisplay
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

# Define hotkeys
bindkey '^G' ai_fetch_suggestion  # Ctrl + G to request suggestions
bindkey '^F' ai_accept_suggestion # Ctrl + F to accept suggestions
bindkey '^B' ai_clear_suggestion  # Ctrl + B to dismiss suggestions