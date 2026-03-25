# --- AI Terminal Suggestion Script ---
AI_LAST_SUGGESTION=""
AI_LAST_REVERSIBILITY=""
AI_LAST_DESCRIPTION=""
AI_BUFFER_OWNER=""

ai_reversibility_color() {
  case "$1" in
    Full)         echo "fg=22"  ;;  # dark green
    Mostly)       echo "fg=23"  ;;  # dark cyan
    Partial)      echo "fg=58"  ;;  # dark yellow
    Hard)         echo "fg=88"  ;;  # dark red
    Irreversible) echo "fg=124" ;;  # deeper red
    *)            echo "fg=8"   ;;  # gray
  esac
}

ai_fetch_suggestion() {
    export AI_CONTEXT_HISTORY="$(history -n -10)"

    local result
    result="$(smart-terminal next-cmd "$BUFFER" 2>/dev/null)"

    AI_LAST_SUGGESTION="$(echo "$result" | sed -n '1p')"
    AI_LAST_DESCRIPTION="$(echo "$result" | sed -n '2p')"
    AI_LAST_REVERSIBILITY="$(echo "$result" | sed -n '3p')"
    AI_BUFFER_OWNER="$BUFFER"
    zle redisplay
}

ai_accept_suggestion() {
  if [[ -n "$AI_LAST_SUGGESTION" ]]; then
    BUFFER="$AI_LAST_SUGGESTION"
    CURSOR=${#BUFFER}
    print -s "$AI_LAST_SUGGESTION"
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
  AI_LAST_SUGGESTION=""
  AI_LAST_REVERSIBILITY=""
  AI_LAST_DESCRIPTION=""
  AI_BUFFER_OWNER=""
  POSTDISPLAY=""
  region_highlight=()
  zle redisplay
}

ai_ghost() {
  region_highlight=("${(@)region_highlight:#*ghost_highlight*}")

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
    local display_text=""
    local desc_text=""

    if [[ "$AI_LAST_SUGGESTION" == "$BUFFER"* ]]; then
      display_text="${AI_LAST_SUGGESTION#$BUFFER}"
    else
      display_text=" -> $AI_LAST_SUGGESTION"
    fi

    if [[ -n "$AI_LAST_DESCRIPTION" ]]; then
      desc_text="  # $AI_LAST_DESCRIPTION"
    fi

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

zle -N ai_fetch_suggestion
zle -N ai_accept_suggestion
zle -N ai_clear_suggestion
zle -N ai_ghost

autoload -Uz add-zle-hook-widget
add-zle-hook-widget line-pre-redraw ai_ghost

bindkey '^G' ai_fetch_suggestion
bindkey '^F' ai_accept_suggestion
bindkey '^B' ai_clear_suggestion