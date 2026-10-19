"""Headline-only analysis. This module has no order, wallet or exchange capability."""
import hashlib
import math
import re

ALIASES = {
    "BTC": ["bitcoin", "btc"], "ETH": ["ethereum", "ether", "eth"],
    "SOL": ["solana", "sol"], "BNB": ["binance coin", "bnb"],
    "XRP": ["ripple", "xrp"], "DOGE": ["dogecoin", "doge"],
    "ADA": ["cardano", "ada"], "AVAX": ["avalanche", "avax"],
}
POSITIVE = {"approval", "approved", "surge", "growth", "rally", "adoption", "gains", "upgrade", "record", "bullish", "inflows"}
NEGATIVE = {"hack", "fraud", "crash", "loss", "losses", "ban", "lawsuit", "exploit", "outflows", "bearish", "falls"}


def fingerprint(title):
    normalized = " ".join(re.findall(r"\w+", title.casefold()))
    return hashlib.sha256(normalized.encode()).hexdigest()


def analyze(title):
    tokens = re.findall(r"\w+", title.casefold())
    assets = [symbol for symbol, names in ALIASES.items()
              if any(re.search(r"\b" + re.escape(name) + r"\b", title.casefold()) for name in names)]
    score = 0
    for index, word in enumerate(tokens):
        value = (word in POSITIVE) - (word in NEGATIVE)
        if index and tokens[index - 1] in {"no", "not", "never"}:
            value *= -1
        score += value
    return {"assets": assets, "sentiment": max(-1.0, min(1.0, score / 3)),
            "method": "headline-lexicon-v1", "confidence": 0.25,
            "impact": "Sinal textual exploratório; não mede efeito causal no preço",
            "model": None, "can_place_orders": False}


def validate_ai(value, fallback, model):
    score = float(value["sentiment"])
    if not math.isfinite(score) or not -1 <= score <= 1:
        raise ValueError("invalid sentiment")
    return {**fallback, "sentiment": score, "method": "ai-headline-v1",
            "model": model, "impact": str(value.get("impact", ""))[:500],
            "confidence": min(1.0, max(0.0, float(value.get("confidence", 0))))}
