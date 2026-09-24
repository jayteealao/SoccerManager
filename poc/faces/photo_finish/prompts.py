"""Prompt sets for the photo finish: one embedding set per (age band, eye
colour). Keeping age and eye colour in the prompt stops the finish from
making older faces younger and from repainting eye colour. Ancestry is never
put in the prompt; geometry, skin tone and low strength carry it.
"""

BASE = ("studio headshot photograph of a {age}-year-old professional footballer, {eyes} eyes, "
        "{detail}natural skin texture, soft studio lighting, neutral grey background, sharp focus")
NEG = ("cartoon, 3d render, cgi, painting, illustration, plastic skin, waxy, doll, blurry, deformed, "
       "distorted face, extra eyes, makeup, text, watermark, lowres{extra}")

AGES = {
    19: ("", ", beard, old"),
    30: ("", ""),
    60: ("wrinkles, crow's feet, forehead lines, eye bags, sagging jowls, ", ", young, youthful, smooth skin"),
}
EYES = ["brown", "hazel", "green", "blue"]


def prompt_sets():
    for age, (detail, extra) in AGES.items():
        for eyes in EYES:
            yield f"age{age}_{eyes}", BASE.format(age=age, eyes=eyes, detail=detail), NEG.format(extra=extra)
