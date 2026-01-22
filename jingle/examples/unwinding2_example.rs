// Example demonstrating the new Unwinding analysis using CPA traits
//
// This example shows how to use the Unwinding analysis to bound loop iterations
// by tracking back-edge visits.

use jingle::analysis::Analysis;
use jingle::analysis::back_edge::BackEdgeCPA;
use jingle::analysis::direct_location::DirectLocationAnalysis;
use jingle::analysis::unwinding2::{UnwindExt, Unwinding};

fn main() {
    println!("Unwinding Analysis Example");
    println!("==========================\n");

    // Example 1: Using Unwinding with DirectLocationAnalysis
    println!("Example 1: Wrapping DirectLocationAnalysis");
    println!("  let location_analysis = DirectLocationAnalysis::new(...);");
    println!("  let unwinding_analysis = Unwinding::new(location_analysis, 3);");
    println!("  // This will bound all back-edges to at most 3 iterations\n");

    // Example 2: Using the UnwindExt trait
    println!("Example 2: Using the .unwind() extension method");
    println!("  let analysis = DirectLocationAnalysis::new(...)");
    println!("    .unwind(5);");
    println!("  // Creates an unwinding analysis with bound of 5\n");

    // Example 3: Using with BackEdgeCPA
    println!("Example 3: Wrapping BackEdgeCPA");
    println!("  let back_edge_analysis = BackEdgeCPA::new();");
    println!("  let unwinding_analysis = Unwinding::new(back_edge_analysis, 10);");
    println!("  // Bounds back-edges to 10 iterations\n");

    println!("Design Overview:");
    println!("================");
    println!("The Unwinding analysis is a compound CPA with two components:");
    println!("1. BackEdgeCountState - tracks visited locations and back-edge counts");
    println!("2. A location analysis (L) - any CPA with LocationState");
    println!();
    println!("The BackEdgeCountState gets strengthened by the location analysis,");
    println!("updating its location tracking and incrementing back-edge counts.");
    println!("When any back-edge count reaches the bound, the analysis terminates.");
    println!();
    println!("Key Features:");
    println!("- Uses the Strengthen trait to make back-edge counting location-sensitive");
    println!("- Terminates exploration when back-edge limit is reached");
    println!("- Can wrap any LocationState-based CPA");
    println!("- Provides .unwind(bound) extension method for convenience");
}
