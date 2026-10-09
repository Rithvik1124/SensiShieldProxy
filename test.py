from pathlib import Path
from docx import Document
from collections import Counter

# Import YOUR actual implementation
from extractTextFromPDF import (
    detect_pii,
    score_legal_text,
    scan_text,
)


CLEAN_DIR = Path("clean")
SENSITIVE_DIR = Path("sensitive")


# =========================================================
# DOCX TEXT EXTRACTION
# =========================================================

def extract_docx_text(path):
    doc = Document(path)

    text = []

    for paragraph in doc.paragraphs:
        if paragraph.text.strip():
            text.append(paragraph.text)

    return "\n".join(text)


# =========================================================
# TEST ONE DOCUMENT
# =========================================================

def test_document(path):
    text = extract_docx_text(path)

    # Use your actual scan_text() implementation
    result = scan_text(text)

    return result


# =========================================================
# TEST DIRECTORY
# =========================================================

def test_directory(directory, expected):
    total = 0
    correct = 0

    entity_counts = Counter()

    for path in sorted(directory.glob("*.docx")):

        total += 1

        result = test_document(path)

        actual = result["decision"]

        for entity in result["pii"]:
            entity_counts[entity] += 1

        if actual == expected:
            correct += 1
            status = "CORRECT"
        else:
            status = "WRONG"

        print(
            f"{path.name:25} "
            f"expected={expected:5} "
            f"actual={actual:5} "
            f"legal={result['legal_score']:.2f} "
            f"PII={result['pii']} "
            f"{status}"
        )

    return {
        "total": total,
        "correct": correct,
        "incorrect": total - correct,
        "entities": entity_counts,
    }


# =========================================================
# MAIN
# =========================================================

def main():

    print("=" * 70)
    print("DLP PROXY TEST")
    print("=" * 70)

    print("\n--- CLEAN DOCUMENTS ---\n")

    clean = test_directory(
        CLEAN_DIR,
        expected="allow"
    )

    print("\n--- SENSITIVE DOCUMENTS ---\n")

    sensitive = test_directory(
        SENSITIVE_DIR,
        expected="block"
    )

    # =====================================================
    # CONFUSION MATRIX
    # =====================================================

    true_negatives = clean["correct"]
    false_positives = clean["incorrect"]

    true_positives = sensitive["correct"]
    false_negatives = sensitive["incorrect"]

    total = (
        true_positives
        + true_negatives
        + false_positives
        + false_negatives
    )

    accuracy = (
        (true_positives + true_negatives) / total
        if total
        else 0
    )

    precision = (
        true_positives /
        (true_positives + false_positives)
        if (true_positives + false_positives)
        else 0
    )

    recall = (
        true_positives /
        (true_positives + false_negatives)
        if (true_positives + false_negatives)
        else 0
    )

    f1 = (
        2 * precision * recall /
        (precision + recall)
        if (precision + recall)
        else 0
    )

    false_positive_rate = (
        false_positives /
        (false_positives + true_negatives)
        if (false_positives + true_negatives)
        else 0
    )

    false_negative_rate = (
        false_negatives /
        (false_negatives + true_positives)
        if (false_negatives + true_positives)
        else 0
    )

    # =====================================================
    # RESULTS
    # =====================================================

    print("\n")
    print("=" * 70)
    print("RESULTS")
    print("=" * 70)

    print(f"\nTrue Positives:      {true_positives}")
    print(f"True Negatives:      {true_negatives}")
    print(f"False Positives:     {false_positives}")
    print(f"False Negatives:     {false_negatives}")

    print("\n" + "-" * 70)

    print(f"Accuracy:            {accuracy:.2%}")
    print(f"Precision:           {precision:.2%}")
    print(f"Recall:              {recall:.2%}")
    print(f"F1 Score:            {f1:.2%}")

    print(f"\nFalse Positive Rate: {false_positive_rate:.2%}")
    print(f"False Negative Rate: {false_negative_rate:.2%}")

    print("\n" + "-" * 70)

    print("PII detected in CLEAN documents:")

    for entity, count in clean["entities"].items():
        print(f"  {entity}: {count}")

    print("\nPII detected in SENSITIVE documents:")

    for entity, count in sensitive["entities"].items():
        print(f"  {entity}: {count}")

    print("\n" + "=" * 70)


if __name__ == "__main__":
    main()