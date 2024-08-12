import { Slot } from "@radix-ui/react-slot";
import * as React from "react";
import { type VariantProps, tv } from "tailwind-variants";

const buttonVariants = tv({
  base: "inline-flex items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 font-bold font-grotesk",
  variants: {
    variant: {
      default: "bg-primary text-primary-foreground hover:bg-primary/90",
      danger: "bg-danger text-danger-foreground hover:bg-danger/90",
      outline: "border border-input bg-transparent hover:bg-accent hover:text-accent-foreground",
      secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
      ghost: "hover:bg-accent hover:text-accent-foreground",
      flat: "bg-primary/20 text-primary",
      link: "font-bold text-primary-300 font-inter",
    },
    size: {
      default: "h-10 px-4 py-2",
      sm: "h-9 rounded-lg px-3",
      lg: "h-11 rounded-lg px-8",
      icon: "h-6 w-6",
      none: "h-fit w-fit p-0",
    },
  },
  defaultVariants: {
    variant: "default",
    size: "default",
  },
});

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  /**
   * When true, the button will render as a `Slot` component.
   * @default false
   */
  asChild?: boolean;
  /**
   * When true, the button will be disabled.
   * @default false
   */
  isDisabled?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "default", asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return <Comp className={buttonVariants({ size, variant, className })} ref={ref} {...props} />;
  },
);

Button.displayName = "Button";

export { Button, buttonVariants };
